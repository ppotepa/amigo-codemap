#[derive(Debug)]
pub struct PatchFile {
    pub old_path: String,
    pub new_path: String,
    pub added_lines: Vec<usize>,
}

pub fn parse_patch_files(text: &str) -> Vec<PatchFile> {
    let mut files = Vec::new();
    let mut current: Option<PatchFile> = None;
    let mut current_line = 0usize;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            if let Some(file) = current.take() {
                files.push(file);
            }
            let parts = rest.split(" b/").collect::<Vec<_>>();
            if parts.len() == 2 {
                current = Some(PatchFile {
                    old_path: parts[0].to_string(),
                    new_path: parts[1].to_string(),
                    added_lines: Vec::new(),
                });
            }
            continue;
        }

        if line.starts_with("@@") {
            if let Some(pos) = line.find('+') {
                let rest = &line[pos + 1..];
                current_line = rest
                    .split([',', ' '])
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0);
            }
            continue;
        }

        if let Some(file) = current.as_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                file.added_lines.push(current_line);
                current_line += 1;
            } else if line.starts_with('-') {
            } else {
                current_line += 1;
            }
        }
    }

    if let Some(file) = current.take() {
        files.push(file);
    }
    files
}
