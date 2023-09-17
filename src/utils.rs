/// take an element from a vec
pub fn take_from_vec<T>(mut vec: Vec<T>, i: usize) -> Option<T> {
    if vec.get(i).is_none() {
        None
    } else {
        Some(vec.swap_remove(i))
    }
}

pub fn format_duration(duration: i64) -> String {
    let h = (duration / 60) / 60;
    let m = (duration / 60) % 60;
    let s = duration % 60;
    if h != 0 {
        format!("{}:{:02}:{:02}s", h, m, s)
    } else if m != 0 {
        format!("{}:{:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}
