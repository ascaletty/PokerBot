use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod poker_server {
    use std::ffi::c_float;

    use pyo3::prelude::*;

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn calc_hand_prob(hand: Vec<String>, community: Vec<String>) -> PyResult<i8> {
            
    }
    fn create_all_hands(hand: Vec<String>, community: Vec<String>){
        let v_1= vec!["Ah"]
    }

}
