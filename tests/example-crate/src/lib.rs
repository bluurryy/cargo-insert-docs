//! Use the [Image] type to load images.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! - **`std`** *(enabled by default)* — Enables loading [`Image`]s
//!   from [`std::io::Read`].
//!
//! ## Image formats
//! The following formats are supported.
//!
//! - **`jpg`** *(enabled by default)* — Enables support for jpg images
//! - **`png`** — Enables support for png images
//! <!-- feature documentation end -->
//!
//! # Examples
//! ```
//! # use example_crate::Image;
//! let image = Image::load("cat.png");
//! # println!("this won't show up in the readme");
//! ```

pub struct Image;

impl Image {
    pub fn load(_: &str) -> Image {
        Image
    }
}
