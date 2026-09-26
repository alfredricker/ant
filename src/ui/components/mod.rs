//! Pieces used by more than one page. Each has a stylesheet of the same name
//! in `assets/styles/`.
mod avatar;
mod brand;
mod footer;
mod post_card;
mod top_bar;

pub use avatar::Avatar;
pub use brand::Brand;
pub use footer::Footer;
pub use post_card::PostCard;
pub use top_bar::TopBar;
