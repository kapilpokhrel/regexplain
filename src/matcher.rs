pub struct RegexMatchGrp {
    pub start_offset: usize,
    pub end_offset: usize,
    pub label: String, // computed label (name or 'group N')
    pub groups: Vec<RegexMatchGrp>,
}

impl RegexMatchGrp {
    fn contains(&self, other: &RegexMatchGrp) -> bool {
        self.start_offset <= other.start_offset && other.end_offset <= self.end_offset
    }

    pub fn contains_offset(&self, offset: usize) -> bool {
        offset >= self.start_offset && offset < self.end_offset
    }

    pub fn insert(&mut self, child: RegexMatchGrp) {
        for sub in &mut self.groups {
            if sub.contains(&child) {
                sub.insert(child);
                return;
            }
        }
        self.groups.push(child);
    }

    pub fn find_group_path(&self, offset: usize) -> Vec<&RegexMatchGrp>
    {
        if !self.contains_offset(offset) {
            return Vec::new();
        }

        let mut path = vec![self];
        for sub in &self.groups {
            if sub.contains_offset(offset) {
                path.extend(sub.find_group_path(offset));
                return path;
            }
        }
        path
    }
}

pub fn eval_regex(re: &regex::bytes::Regex, text: &str) -> Vec<RegexMatchGrp> {
    let grp_names: Vec<Option<&str>> = re.capture_names().collect();
    let mut matches: Vec<RegexMatchGrp> = Vec::new();
    for caps in re.captures_iter(text.as_bytes()) {
        let full_match = caps.get(0).unwrap();
        let mut root_match = RegexMatchGrp {
            start_offset: full_match.start(),
            end_offset: full_match.end(),
            label: String::new(),
            groups: Vec::new(),
        };

        for (i, grp) in caps.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let name = grp_names.get(i).and_then(|n| n.and_then(|n| Some(n.to_string())));
            if let Some(grp_match) = grp {
                let child = RegexMatchGrp {
                    start_offset: grp_match.start(),
                    end_offset: grp_match.end(),
                    label: if let Some(ref n) = name { n.clone() } else { format!("group {}", i) },
                    groups: Vec::new(),
                };
                root_match.insert(child);
            }
        }

        matches.push(root_match);
    }
    matches
}
