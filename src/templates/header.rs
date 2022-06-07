pub use HeaderTemplate as Header;

use crate::templates::base::self_ref;

markup::define! {
    HeaderTemplate(url: std::rc::Rc<url::Url>) {
        div {
            h1 {
                @self_ref("./", "SearProxy")
            }
            p {
                "This is a proxified and sanitized version, visit "
                @self_ref(url.as_str(),  "original page")
                "."
            }
        }
    }
}
