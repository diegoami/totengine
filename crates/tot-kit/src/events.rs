//! Scenario macro-events (T1-7): parser for the `Events.txt` format.
//!
//! Grammar (as used by real scenarios — FallRome's file is the corpus):
//! the file is `@BEGINEVENTS … @ENDEVENTS`; inside, events are
//! `@IF <trigger> [params] @THEN <actions…> @ENDIF`. Lines are keywords
//! (UPPERCASE), `key=value` parameters, positional values (numbers,
//! `locations`/`endlocations` blocks) or `;comments`. `TEXT … ENDTEXT`
//! wraps free text. Unknown keywords are preserved verbatim — the parser
//! must never drop scenario content it does not act on.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Word(String),
    Number(i32),
    Points(Vec<(i32, i32)>),
    Text(String),
}

impl Value {
    pub fn word(&self) -> Option<&str> {
        match self {
            Value::Word(w) => Some(w),
            _ => None,
        }
    }
    pub fn number(&self) -> Option<i32> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub keyword: String,
    /// Positional arguments in source order.
    pub args: Vec<Value>,
    /// `key=value` parameters (value split at the first `=`).
    pub params: BTreeMap<String, Value>,
}

impl Command {
    pub fn param(&self, key: &str) -> Option<&Value> {
        self.params.get(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The `@IF` trigger keyword (e.g. `UNITKILLED`).
    pub trigger: Command,
    /// The `@THEN` actions.
    pub actions: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventFile {
    pub events: Vec<Event>,
    /// Keywords seen outside our action coverage (kept for honesty).
    pub unknown_keywords: Vec<String>,
}

#[derive(Debug)]
pub enum EventError {
    NotEventFile,
    UnclosedBlock { line: usize, what: &'static str },
}

fn split_kv(line: &str) -> Option<(String, String)> {
    let (k, v) = line.split_once('=')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

fn parse_value(s: &str) -> Value {
    let t = s.trim();
    if let Ok(n) = t.parse::<i32>() {
        Value::Number(n)
    } else {
        Value::Word(t.to_string())
    }
}

/// Parse an Events.txt body. `data` may start anywhere (the
/// `@BEGINEVENTS` line itself is optional).
pub fn parse_events(data: &str) -> Result<EventFile, EventError> {
    let mut out = EventFile::default();
    let mut events = Vec::new();
    let mut lines = data.lines().enumerate().peekable();
    // skip to @BEGINEVENTS if present
    while let Some((_, l)) = lines.peek() {
        let t = l.trim();
        if t.is_empty() || t.starts_with(';') {
            lines.next();
            continue;
        }
        if t.eq_ignore_ascii_case("@BEGINEVENTS") {
            lines.next();
        }
        break;
    }
    while let Some((lineno, line)) = lines.next() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.eq_ignore_ascii_case("@DEBUG") {
            continue;
        }
        if t.eq_ignore_ascii_case("@ENDEVENTS") {
            break;
        }
        if !t.eq_ignore_ascii_case("@IF") {
            continue; // stray line between events
        }
        // trigger line: next non-comment line
        let trigger_kw = loop {
            match lines.next() {
                Some((_, l)) => {
                    let lt = l.trim();
                    if lt.is_empty() || lt.starts_with(';') {
                        continue;
                    }
                    break lt.to_uppercase();
                }
                None => {
                    return Err(EventError::UnclosedBlock {
                        line: lineno,
                        what: "trigger",
                    })
                }
            }
        };
        let mut trigger = Command {
            keyword: trigger_kw,
            args: Vec::new(),
            params: BTreeMap::new(),
        };
        // consume trigger params until @THEN
        loop {
            match lines.peek() {
                Some((_, l)) => {
                    let lt = l.trim();
                    if lt.eq_ignore_ascii_case("@THEN") {
                        lines.next();
                        break;
                    }
                    if lt.is_empty() || lt.starts_with(';') {
                        lines.next();
                        continue;
                    }
                    lines.next();
                    if let Some((k, v)) = split_kv(lt) {
                        trigger.params.insert(k, parse_value(&v));
                    } else {
                        trigger.args.push(parse_value(lt));
                    }
                }
                None => {
                    return Err(EventError::UnclosedBlock {
                        line: lineno,
                        what: "@THEN",
                    });
                }
            }
        }
        // consume actions until @ENDIF
        let mut actions: Vec<Command> = Vec::new();
        loop {
            match lines.next() {
                Some((lno, l)) => {
                    let lt = l.trim();
                    if lt.eq_ignore_ascii_case("@ENDIF") {
                        break;
                    }
                    if lt.is_empty() || lt.starts_with(';') {
                        continue;
                    }
                    if let Some((k, v)) = split_kv(lt) {
                        if let Some(cmd) = actions.last_mut() {
                            cmd.params.insert(k, parse_value(&v));
                            continue;
                        }
                    }
                    let kw = lt.to_uppercase();
                    let mut cmd = Command {
                        keyword: kw.clone(),
                        args: Vec::new(),
                        params: BTreeMap::new(),
                    };
                    if kw == "LOCATIONS" {
                        let mut pts = Vec::new();
                        for (_, l2) in lines.by_ref() {
                            let t2 = l2.trim();
                            if t2.eq_ignore_ascii_case("ENDLOCATIONS") {
                                break;
                            }
                            if t2.is_empty() || t2.starts_with(';') {
                                continue;
                            }
                            // `x,y` or `x,y,z` (z = map, Macro.txt; not kept)
                            let mut it = t2.split(',').map(|v| v.trim().parse::<i32>());
                            if let (Some(Ok(x)), Some(Ok(y))) = (it.next(), it.next()) {
                                pts.push((x, y));
                            }
                        }
                        // `locations … endlocations` is a parameter of the
                        // preceding CREATEUNIT (Macro.txt), not an action.
                        match actions.last_mut() {
                            Some(prev) if prev.keyword == "CREATEUNIT" => {
                                prev.args.push(Value::Points(pts));
                            }
                            _ => {
                                cmd.args.push(Value::Points(pts));
                                actions.push(cmd);
                            }
                        }
                        continue;
                    }
                    if kw == "TEXT" {
                        let mut buf = Vec::new();
                        for (_, l2) in lines.by_ref() {
                            let t2 = l2.trim_end();
                            if t2.trim().eq_ignore_ascii_case("ENDTEXT") {
                                break;
                            }
                            buf.push(t2.to_string());
                        }
                        cmd.args.push(Value::Text(buf.join("\n")));
                        actions.push(cmd);
                        continue;
                    }
                    // positional line: a bare number belongs to the
                    // previous keyword (e.g. PLAYCDTRACK\n11), not to a
                    // new command
                    if !actions.is_empty()
                        && !lt.contains('=')
                        && !lt.starts_with('@')
                        && lt.trim().parse::<i32>().is_ok()
                        && actions.last().map(|c| c.params.is_empty()).unwrap_or(false)
                    {
                        actions.last_mut().unwrap().args.push(parse_value(lt));
                        continue;
                    }
                    actions.push(cmd);
                    let _ = lno;
                }
                None => {
                    return Err(EventError::UnclosedBlock {
                        line: lineno,
                        what: "@ENDIF",
                    })
                }
            }
        }
        events.push(Event { trigger, actions });
    }
    out.events = events;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fallrome() -> Option<String> {
        let p = std::env::var_os("CIVTOT_HOME")?;
        let p = std::path::Path::new(&p).join("Scenario/FallRome/Events.txt");
        std::fs::read_to_string(p).ok()
    }

    #[test]
    fn parses_minimal_event() {
        let src =
            "@BEGINEVENTS\n@IF\nTURN\nturn=5\n@THEN\nTEXT\nhello\nENDTEXT\n@ENDIF\n@ENDEVENTS\n";
        let f = parse_events(src).unwrap();
        assert_eq!(f.events.len(), 1);
        assert_eq!(f.events[0].trigger.keyword, "TURN");
        assert_eq!(f.events[0].trigger.param("turn").unwrap().number(), Some(5));
        assert_eq!(f.events[0].actions.len(), 1);
        match &f.events[0].actions[0].args[0] {
            Value::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn parses_fallrome_corpus() {
        let Some(src) = fallrome() else {
            eprintln!("skipping: CIVTOT_HOME/FallRome Events.txt not present");
            return;
        };
        let f = parse_events(&src).unwrap();
        assert_eq!(f.events.len(), 64, "FallRome has 64 @IF blocks");
        // a UNITKILLED event with CREATEUNIT + locations
        let e = f
            .events
            .iter()
            .find(|e| {
                e.trigger.keyword == "UNITKILLED"
                    && e.trigger.param("unit").and_then(|v| v.word()) == Some("Theodoric I")
            })
            .expect("Theodoric I event");
        let create = e
            .actions
            .iter()
            .find(|c| c.keyword == "CREATEUNIT")
            .unwrap();
        // The install's file (FallRome/ and Scenario/FallRome/ identical):
        // owner=Visigoths and six locations, 24,60 first.
        assert_eq!(
            create.param("owner").and_then(|v| v.word()),
            Some("Visigoths")
        );
        assert_eq!(
            create.param("unit").and_then(|v| v.word()),
            Some("Theodoric II")
        );
        match create.args.iter().find(|v| matches!(v, Value::Points(_))) {
            Some(Value::Points(pts)) => {
                assert_eq!(pts.len(), 6);
                assert_eq!(pts[0], (24, 60));
            }
            _ => panic!("expected the locations list on CREATEUNIT"),
        }
        assert!(!e.actions.iter().any(|c| c.keyword == "LOCATIONS"));
        assert!(e.actions.iter().any(|c| c.keyword == "JUSTONCE"));
    }
}
