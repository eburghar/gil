/// take an element from a vec
pub fn take_from_vec<T>(mut vec: Vec<T>, i: usize) -> Option<T> {
	if vec.get(i).is_none() {
		None
	} else {
		Some(vec.swap_remove(i))
	}
}
