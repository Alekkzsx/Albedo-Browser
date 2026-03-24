use super::percent_encoding::{self, EncodeSet};

#[derive(Debug, Clone, Default)]
pub struct UrlSearchParams {
    params: Vec<(String, String)>,
}

impl UrlSearchParams {
    pub fn new(query: Option<&str>) -> Self {
        let mut search_params = Self::default();
        if let Some(q) = query {
            let q = q.trim_start_matches('?');
            for pair in q.split('&') {
                if pair.is_empty() { continue; }
                let mut parts = pair.splitn(2, '=');
                let key = percent_encoding::decode(parts.next().unwrap_or(""));
                let val = percent_encoding::decode(parts.next().unwrap_or(""));
                search_params.append(&key, &val);
            }
        }
        search_params
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.params.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    }

    pub fn get_all(&self, name: &str) -> Vec<String> {
        self.params.iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn set(&mut self, name: &str, value: &str) {
        if let Some(pos) = self.params.iter().position(|(k, _)| k == name) {
            self.params[pos].1 = value.to_string();
            let mut i = pos + 1; // Start checking from the element after the first match
            while i < self.params.len() {
                if self.params[i].0 == name {
                    self.params.remove(i);
                } else {
                    i += 1;
                }
            }
        } else {
            self.append(name, value);
        }
    }

    pub fn append(&mut self, name: &str, value: &str) {
        self.params.push((name.to_string(), value.to_string()));
    }

    pub fn delete(&mut self, name: &str) {
        self.params.retain(|(k, _)| k != name);
    }

    pub fn has(&self, name: &str) -> bool {
        self.params.iter().any(|(k, _)| k == name)
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for (i, (k, v)) in self.params.iter().enumerate() {
            if i > 0 { result.push('&'); }
            result.push_str(&percent_encoding::encode(k, EncodeSet::Query));
            result.push('=');
            result.push_str(&percent_encoding::encode(v, EncodeSet::Query));
        }
        result
    }

    pub fn entries(&self) -> impl Iterator<Item = &(String, String)> {
        self.params.iter()
    }

    pub fn sort(&mut self) {
        self.params.sort_by(|a, b| a.0.cmp(&b.0));
    }
}
