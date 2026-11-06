//! Lecteur minimal du format texte KeyValues (VDF/ACF) de Valve.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Obj(Obj),
}

/// Objet VDF. Les clés sont gardées telles quelles ; `get` les compare sans casse,
/// car Steam écrit parfois `apps` et parfois `Apps`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Obj(pub BTreeMap<String, Value>);

impl Obj {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key).or_else(|| {
            self.0
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v)
        })
    }
    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Str(s) => Some(s),
            Value::Obj(_) => None,
        }
    }
    pub fn obj(&self, key: &str) -> Option<&Obj> {
        match self.get(key)? {
            Value::Obj(o) => Some(o),
            Value::Str(_) => None,
        }
    }
    /// Suit un chemin de clés : `o.path(&["a", "b"])`.
    pub fn path(&self, keys: &[&str]) -> Option<&Obj> {
        keys.iter().try_fold(self, |o, k| o.obj(k))
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq)]
enum Tok {
    Str(String),
    Open,
    Close,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Tok::Open),
            '}' => out.push(Tok::Close),
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(o) => s.push(o),
                            None => return Err("fin de fichier dans une chaîne".into()),
                        },
                        Some('"') => break,
                        Some(o) => s.push(o),
                        None => return Err("chaîne non fermée".into()),
                    }
                }
                out.push(Tok::Str(s));
            }
            '/' if chars.peek() == Some(&'/') => {
                for o in chars.by_ref() {
                    if o == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                // Clé sans guillemets (rare, mais tolérée).
                let mut s = String::from(c);
                while let Some(&o) = chars.peek() {
                    if o.is_whitespace() || o == '{' || o == '}' || o == '"' {
                        break;
                    }
                    s.push(o);
                    chars.next();
                }
                out.push(Tok::Str(s));
            }
        }
    }
    Ok(out)
}

pub fn parse(src: &str) -> Result<Obj, String> {
    let toks = tokenize(src.trim_start_matches('\u{feff}'))?;
    let mut pos = 0;
    let obj = parse_obj(&toks, &mut pos, true)?;
    Ok(obj)
}

fn parse_obj(toks: &[Tok], pos: &mut usize, top: bool) -> Result<Obj, String> {
    let mut map = BTreeMap::new();
    loop {
        match toks.get(*pos) {
            None if top => return Ok(Obj(map)),
            None => return Err("accolade non fermée".into()),
            Some(Tok::Close) if !top => {
                *pos += 1;
                return Ok(Obj(map));
            }
            Some(Tok::Str(key)) => {
                *pos += 1;
                let val = match toks.get(*pos) {
                    Some(Tok::Str(v)) => {
                        *pos += 1;
                        Value::Str(v.clone())
                    }
                    Some(Tok::Open) => {
                        *pos += 1;
                        Value::Obj(parse_obj(toks, pos, false)?)
                    }
                    _ => return Err(format!("valeur manquante pour « {key} »")),
                };
                map.insert(key.clone(), val);
            }
            Some(t) => return Err(format!("jeton inattendu : {t:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_escapes() {
        let o = parse(
            "// commentaire\n\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n}\n",
        )
        .unwrap();
        let p = o.path(&["libraryfolders", "0"]).unwrap().str("path");
        assert_eq!(p, Some("C:\\Program Files (x86)\\Steam"));
    }

    #[test]
    fn case_insensitive_get() {
        let o = parse("\"A\" { \"Apps\" { \"1\" \"x\" } }").unwrap();
        assert!(o.path(&["a", "apps"]).is_some());
    }

    #[test]
    fn rejects_unclosed() {
        assert!(parse("\"a\" { \"b\" \"c\"").is_err());
    }
}
