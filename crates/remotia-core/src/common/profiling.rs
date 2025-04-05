#[macro_export]
macro_rules! vec_avg {
    ($data_vec:expr, $data_type:ty) => {
        $data_vec.iter().sum::<$data_type>() / $data_vec.len() as $data_type
    };
}

#[macro_export]
macro_rules! field_vec {
    ($data_vec:expr, $field_name:ident, $data_type:ty) => {
        $data_vec
            .iter()
            .map(|o| o.$field_name)
            .collect::<Vec<$data_type>>()
    };
}
