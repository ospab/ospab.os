use std::collections::HashMap;

pub fn parse_toml(content: &str) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    let mut current_section = String::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key_part = line[..eq_pos].trim();
            let value_part = line[eq_pos + 1..].trim();

            let key = if current_section.is_empty() {
                key_part.to_string()
            } else {
                format!("{}.{}", current_section, key_part)
            };

            let value = if value_part.starts_with('"') && value_part.ends_with('"') {
                value_part[1..value_part.len() - 1].to_string()
            } else if value_part.starts_with('\'') && value_part.ends_with('\'') {
                value_part[1..value_part.len() - 1].to_string()
            } else {
                value_part.to_string()
            };

            map.insert(key, value);
        } else {
            return Err(format!("Invalid line {}: {}", line_num + 1, line));
        }
    }

    Ok(map)
}

pub fn serialize_toml(map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let mut sections: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (key, value) in map {
        if let Some(dot_pos) = key.find('.') {
            let section = &key[..dot_pos];
            let subkey = &key[dot_pos + 1..];
            sections.entry(section.to_string()).or_insert(Vec::new()).push((subkey.to_string(), value.clone()));
        } else {
            sections.entry("".to_string()).or_insert(Vec::new()).push((key.clone(), value.clone()));
        }
    }

    for (section, keys) in sections {
        if !section.is_empty() {
            output.push_str(&format!("[{}]\n", section));
        }
        for (key, value) in keys {
            output.push_str(&format!("{} = \"{}\"\n", key, value));
        }
        output.push('\n');
    }

    output
}