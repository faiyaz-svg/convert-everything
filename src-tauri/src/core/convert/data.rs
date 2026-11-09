//! Data files (CSV, TSV, JSON, YAML, TOML, XML, spreadsheets) converted inside the app.
//!
//! Everything is read into a JSON value first. Tables become a list of records
//! (one object per row, keyed by the header row).

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value};

use crate::core::options::Opts;

pub fn convert(input: &Path, from: &str, to: &str, o: &Opts, out: &Path) -> Result<()> {
    let value = read(input, from, o)?;
    write(&value, to, o, out)
}

// ------------------------------------------------------------------ reading

fn read_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).context("Could not read the file")?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

pub fn read(path: &Path, from: &str, o: &Opts) -> Result<Value> {
    match from {
        "csv" => {
            let text = read_text(path)?;
            let delim = match o.str_or("delimiter", "auto").as_str() {
                ";" => b';',
                "," => b',',
                _ => guess_delimiter(&text),
            };
            read_delimited(&text, delim, o.bool("types", true))
        }
        "tsv" => read_delimited(&read_text(path)?, b'\t', o.bool("types", true)),
        "json" => serde_json::from_str(&read_text(path)?).map_err(|e| anyhow!("This is not valid JSON ({e}).")),
        "yaml" => serde_yaml::from_str(&read_text(path)?).map_err(|e| anyhow!("This is not valid YAML ({e}).")),
        "toml" => {
            let table: toml::Table = toml::from_str(&read_text(path)?).map_err(|e| anyhow!("This is not valid TOML ({e})."))?;
            Ok(toml_to_json(toml::Value::Table(table)))
        }
        "xml" => xml_to_json(&read_text(path)?),
        "xlsx" | "xls" | "ods" => read_sheet(path, o.num("sheet").unwrap_or(1.0).max(1.0) as usize),
        other => bail!("Can't read {other} as data."),
    }
}

fn guess_delimiter(text: &str) -> u8 {
    let first = text.lines().next().unwrap_or("");
    let count = |c: char| first.matches(c).count();
    let (comma, semi, tab) = (count(','), count(';'), count('\t'));
    if tab > comma && tab > semi {
        b'\t'
    } else if semi > comma {
        b';'
    } else {
        b','
    }
}

/// Header names: empty ones become "column 3", repeated ones get a number.
fn clean_headers(raw: Vec<String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    raw.into_iter()
        .enumerate()
        .map(|(i, h)| {
            let base = if h.trim().is_empty() { format!("column {}", i + 1) } else { h.trim().to_string() };
            let mut name = base.clone();
            let mut n = 2;
            while seen.contains(&name) {
                name = format!("{base} {n}");
                n += 1;
            }
            seen.push(name.clone());
            name
        })
        .collect()
}

fn typed(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() {
        return Value::Null;
    }
    match t {
        "true" | "TRUE" | "True" => return Value::Bool(true),
        "false" | "FALSE" | "False" => return Value::Bool(false),
        _ => {}
    }
    // Keep values like "007" or "+1 555" as text: they are codes, not numbers.
    let leading_zero = t.len() > 1 && t.starts_with('0') && !t.starts_with("0.");
    if !leading_zero && !t.starts_with('+') {
        if let Ok(i) = t.parse::<i64>() {
            return Value::from(i);
        }
        if t.contains('.')
            && let Ok(f) = t.parse::<f64>()
            && f.is_finite()
        {
            return Value::from(f);
        }
    }
    Value::String(s.to_string())
}

fn read_delimited(text: &str, delim: u8, types: bool) -> Result<Value> {
    let mut rdr = csv::ReaderBuilder::new().delimiter(delim).flexible(true).from_reader(text.as_bytes());
    let headers = clean_headers(rdr.headers()?.iter().map(String::from).collect());
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| anyhow!("Could not read the table ({e})."))?;
        let mut m = Map::new();
        for (i, h) in headers.iter().enumerate() {
            let cell = rec.get(i).unwrap_or("");
            m.insert(h.clone(), if types { typed(cell) } else { Value::String(cell.to_string()) });
        }
        rows.push(Value::Object(m));
    }
    Ok(Value::Array(rows))
}

fn read_sheet(path: &Path, number: usize) -> Result<Value> {
    use calamine::{Data, Reader, open_workbook_auto};
    let mut wb = open_workbook_auto(path).map_err(|e| anyhow!("Could not open the spreadsheet ({e})."))?;
    let names = wb.sheet_names().to_vec();
    let name = names.get(number - 1).ok_or_else(|| anyhow!("The spreadsheet has {} sheet(s), so there is no sheet {number}.", names.len()))?;
    let range = wb.worksheet_range(name).map_err(|e| anyhow!("Could not read sheet {name} ({e})."))?;
    let mut rows = range.rows();
    let Some(head) = rows.next() else { return Ok(Value::Array(vec![])) };
    let headers = clean_headers(head.iter().map(|c| c.to_string()).collect());
    let cell = |c: &Data| -> Value {
        match c {
            Data::Int(i) => Value::from(*i),
            Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => Value::from(*f as i64),
            Data::Float(f) => Value::from(*f),
            Data::String(s) => Value::String(s.clone()),
            Data::Bool(b) => Value::Bool(*b),
            Data::DateTime(d) => match d.as_datetime() {
                Some(dt) => Value::String(dt.to_string().trim_end_matches(" 00:00:00").to_string()),
                None => Value::from(d.as_f64()),
            },
            Data::DateTimeIso(s) | Data::DurationIso(s) => Value::String(s.clone()),
            Data::Error(_) | Data::Empty => Value::Null,
        }
    };
    let mut out = Vec::new();
    for row in rows {
        if row.iter().all(|c| matches!(c, Data::Empty)) {
            continue;
        }
        let mut m = Map::new();
        for (i, h) in headers.iter().enumerate() {
            m.insert(h.clone(), row.get(i).map(cell).unwrap_or(Value::Null));
        }
        out.push(Value::Object(m));
    }
    Ok(Value::Array(out))
}

fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        toml::Value::Float(f) => Value::from(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => Value::Object(t.into_iter().map(|(k, v)| (k, toml_to_json(v))).collect()),
    }
}

/// XML to JSON: attributes become "@name", text next to child elements becomes "#text",
/// and repeated child elements become a list.
fn xml_to_json(text: &str) -> Result<Value> {
    let doc = roxmltree::Document::parse(text).map_err(|e| anyhow!("This is not valid XML ({e})."))?;
    fn element(node: roxmltree::Node) -> Value {
        let mut map = Map::new();
        for a in node.attributes() {
            map.insert(format!("@{}", a.name()), Value::String(a.value().to_string()));
        }
        let mut text = String::new();
        for child in node.children() {
            if child.is_element() {
                let key = child.tag_name().name().to_string();
                let v = element(child);
                match map.get_mut(&key) {
                    Some(Value::Array(list)) => list.push(v),
                    Some(existing) => {
                        let first = existing.take();
                        *existing = Value::Array(vec![first, v]);
                    }
                    None => {
                        map.insert(key, v);
                    }
                }
            } else if child.is_text() {
                text.push_str(child.text().unwrap_or(""));
            }
        }
        let text = text.trim();
        if map.is_empty() {
            return if text.is_empty() { Value::Null } else { Value::String(text.to_string()) };
        }
        if !text.is_empty() {
            map.insert("#text".into(), Value::String(text.to_string()));
        }
        Value::Object(map)
    }
    let root = doc.root_element();
    let mut top = Map::new();
    top.insert(root.tag_name().name().to_string(), element(root));
    Ok(Value::Object(top))
}

// ------------------------------------------------------------------ writing

pub fn write(v: &Value, to: &str, o: &Opts, out: &Path) -> Result<()> {
    match to {
        "json" => {
            let s = if o.bool("pretty", true) { serde_json::to_string_pretty(v)? } else { serde_json::to_string(v)? };
            std::fs::write(out, s + "\n")?;
        }
        "yaml" => std::fs::write(out, serde_yaml::to_string(v)?)?,
        "toml" => {
            let top = match strip_nulls(v.clone()) {
                Value::Object(m) => Value::Object(m),
                other @ Value::Array(_) => Value::Object(Map::from_iter([("rows".to_string(), other)])),
                other => Value::Object(Map::from_iter([("value".to_string(), other)])),
            };
            let s = toml::to_string_pretty(&top).map_err(|e| anyhow!("This data can't be written as TOML ({e})."))?;
            std::fs::write(out, s)?;
        }
        "xml" => std::fs::write(out, json_to_xml(v))?,
        "csv" | "tsv" => {
            let delim = if to == "tsv" { b'\t' } else if o.str_or("outDelimiter", ",") == ";" { b';' } else { b',' };
            let (headers, rows) = table(v);
            let mut w = csv::WriterBuilder::new().delimiter(delim).from_writer(Vec::new());
            w.write_record(&headers)?;
            for row in rows {
                w.write_record(row.iter().map(cell_text))?;
            }
            let bytes = w.into_inner().map_err(|e| anyhow!("{e}"))?;
            // Excel reads UTF-8 CSV files correctly only with a byte order mark.
            let mut data = Vec::new();
            if !bytes.is_ascii() {
                data.extend_from_slice(b"\xEF\xBB\xBF");
            }
            data.extend(bytes);
            std::fs::write(out, data)?;
        }
        "xlsx" => write_xlsx(v, out)?,
        other => bail!("Can't write {other} as data."),
    }
    Ok(())
}

fn strip_nulls(v: Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k, strip_nulls(v))).collect()),
        Value::Array(a) => Value::Array(a.into_iter().filter(|v| !v.is_null()).map(strip_nulls).collect()),
        other => other,
    }
}

fn cell_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Finds the list of records in a value and flattens each record into columns.
fn table(v: &Value) -> (Vec<String>, Vec<Vec<Value>>) {
    let records: Vec<Value> = match v {
        Value::Array(a) => a.clone(),
        Value::Object(m) if m.len() == 1 => match m.values().next().unwrap() {
            Value::Array(a) => a.clone(),
            Value::Object(inner) => {
                // XML-style {"root": {"item": [...]}}
                match inner.values().find(|x| x.is_array()) {
                    Some(Value::Array(a)) if inner.len() == 1 => a.clone(),
                    _ => vec![Value::Object(inner.clone())],
                }
            }
            other => vec![other.clone()],
        },
        other => vec![other.clone()],
    };

    // A list of lists: the first one is the header row.
    if !records.is_empty() && records.iter().all(|r| r.is_array()) {
        let rows: Vec<Vec<Value>> = records.iter().map(|r| r.as_array().unwrap().clone()).collect();
        let headers = rows[0].iter().map(cell_text).collect();
        return (headers, rows[1..].to_vec());
    }

    let mut headers: Vec<String> = Vec::new();
    let mut flat_rows = Vec::new();
    for r in &records {
        let mut flat = Map::new();
        match r {
            Value::Object(_) => flatten("", r, &mut flat),
            other => {
                flat.insert("value".into(), other.clone());
            }
        }
        for k in flat.keys() {
            if !headers.contains(k) {
                headers.push(k.clone());
            }
        }
        flat_rows.push(flat);
    }
    let rows = flat_rows.into_iter().map(|m| headers.iter().map(|h| m.get(h).cloned().unwrap_or(Value::Null)).collect()).collect();
    (headers, rows)
}

fn flatten(prefix: &str, v: &Value, out: &mut Map<String, Value>) {
    match v {
        Value::Object(m) => {
            for (k, child) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&key, child, out);
            }
        }
        Value::Array(_) => {
            out.insert(prefix.to_string(), Value::String(v.to_string()));
        }
        other => {
            out.insert(prefix.to_string(), other.clone());
        }
    }
}

fn write_xlsx(v: &Value, out: &Path) -> Result<()> {
    use rust_xlsxwriter::{Format, Workbook};
    let (headers, rows) = table(v);
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let bold = Format::new().set_bold();
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &bold)?;
    }
    for (r, row) in rows.iter().enumerate() {
        let r = (r + 1) as u32;
        for (c, cell) in row.iter().enumerate() {
            let c = c as u16;
            match cell {
                Value::Null => {}
                Value::Bool(b) => {
                    ws.write_boolean(r, c, *b)?;
                }
                Value::Number(n) => {
                    ws.write_number(r, c, n.as_f64().unwrap_or(0.0))?;
                }
                other => {
                    let s = cell_text(other);
                    let s: String = s.chars().take(32_000).collect();
                    ws.write_string(r, c, &s)?;
                }
            }
        }
    }
    ws.autofit();
    wb.save(out)?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn xml_name(s: &str) -> String {
    let mut n: String = s.chars().map(|c| if c.is_alphanumeric() || "_-.".contains(c) { c } else { '_' }).collect();
    if n.is_empty() || !n.chars().next().unwrap().is_alphabetic() && !n.starts_with('_') {
        n.insert(0, '_');
    }
    n
}

fn json_to_xml(v: &Value) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    match v {
        Value::Object(m) if m.len() == 1 && !m.keys().next().unwrap().starts_with('@') && !m.values().next().unwrap().is_array() => {
            let (k, inner) = m.iter().next().unwrap();
            element(&mut out, k, inner, 0);
        }
        Value::Array(items) => {
            out.push_str("<root>\n");
            for item in items {
                element(&mut out, "item", item, 1);
            }
            out.push_str("</root>\n");
        }
        other => element(&mut out, "root", other, 0),
    }
    out
}

fn element(out: &mut String, name: &str, v: &Value, depth: usize) {
    let pad = "  ".repeat(depth);
    let name = xml_name(name);
    match v {
        Value::Array(items) => {
            for item in items {
                element(out, &name, item, depth);
            }
        }
        Value::Object(m) => {
            let mut attrs = String::new();
            let mut text = None;
            let mut children = Vec::new();
            for (k, child) in m {
                if let Some(a) = k.strip_prefix('@')
                    && !child.is_object()
                    && !child.is_array()
                {
                    attrs.push_str(&format!(" {}=\"{}\"", xml_name(a), xml_escape(&cell_text(child))));
                } else if k == "#text" {
                    text = Some(cell_text(child));
                } else {
                    children.push((k, child));
                }
            }
            if children.is_empty() {
                match text {
                    Some(t) => out.push_str(&format!("{pad}<{name}{attrs}>{}</{name}>\n", xml_escape(&t))),
                    None => out.push_str(&format!("{pad}<{name}{attrs}/>\n")),
                }
            } else {
                out.push_str(&format!("{pad}<{name}{attrs}>\n"));
                if let Some(t) = text {
                    out.push_str(&format!("{pad}  {}\n", xml_escape(&t)));
                }
                for (k, child) in children {
                    element(out, k, child, depth + 1);
                }
                out.push_str(&format!("{pad}</{name}>\n"));
            }
        }
        Value::Null => out.push_str(&format!("{pad}<{name}/>\n")),
        other => out.push_str(&format!("{pad}<{name}>{}</{name}>\n", xml_escape(&cell_text(other)))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn csv_values_get_types() {
        let v = read_delimited("name,age,zip,ok\nAda,36,02134,true\n", b',', true).unwrap();
        assert_eq!(v, json!([{"name": "Ada", "age": 36, "zip": "02134", "ok": true}]));
    }

    #[test]
    fn nested_records_flatten_to_columns() {
        let (h, rows) = table(&json!({"people": [{"name": "Ada", "address": {"city": "London"}}, {"name": "Alan", "tags": [1, 2]}]}));
        assert_eq!(h, vec!["name", "address.city", "tags"]);
        assert_eq!(rows[1][2], json!("[1,2]"));
    }

    #[test]
    fn xml_round_trip_keeps_structure() {
        let v = xml_to_json(r#"<books><book id="1"><title>A &amp; B</title></book><book id="2"><title>C</title></book></books>"#).unwrap();
        assert_eq!(v, json!({"books": {"book": [{"@id": "1", "title": "A & B"}, {"@id": "2", "title": "C"}]}}));
        let xml = json_to_xml(&v);
        assert!(xml.contains("<book id=\"1\">"));
        assert!(xml.contains("<title>A &amp; B</title>"));
        assert_eq!(xml_to_json(&xml).unwrap(), v);
    }

    #[test]
    fn delimiter_guess() {
        assert_eq!(guess_delimiter("a;b;c\n1;2;3"), b';');
        assert_eq!(guess_delimiter("a,b\n"), b',');
        assert_eq!(guess_delimiter("a\tb\tc"), b'\t');
    }
}
