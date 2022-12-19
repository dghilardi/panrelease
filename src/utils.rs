pub fn get_range(whole_buffer: &str, part: &str) -> (usize, usize) {
    let start = part.as_ptr() as usize - whole_buffer.as_ptr() as usize;
    let end = start + part.len();
    (start, end)
}