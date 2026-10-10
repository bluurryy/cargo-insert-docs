pub mod skipped_by_cli {
    pub fn function() -> i32 {
        42
    }
}

pub mod skipped_by_workspace {
    pub fn function() -> i32 {
        42
    }
}

pub mod skipped_by_package {
    pub fn function() -> i32 {
        42
    }
}

pub mod not_skipped {
    pub fn function() -> i32 {
        42
    }
}

pub mod nested {
    pub mod skipped_by_package {
        pub fn function() -> i32 {
            42
        }
    }

    pub mod not_skipped {
        pub fn function() -> i32 {
            42
        }
    }
}
