// I'm pretty sure HashMap is Send, as is String
#[derive(Debug, Serialize, Deserialize)]
pub struct Completions {
    // Map<Line, Vec<Path>>
    info: HashMap<String, Vec<(PathBuf, f32)>>
}
