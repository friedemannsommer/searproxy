pub use HeaderTemplate as Header;

use crate::templates::base::self_ref;

markup::define! {
    HeaderTemplate(url: std::rc::Rc<url::Url>) {
        div {
            h1 {
                @self_ref("./", "SearProxy")
                " is neither the owner nor the author of this content."
            }
            p {
                "Scripts are deactivated. Web page appearance may have changed. Visit the "
                @self_ref(url.as_str(),  "original page")
                "."
            }
        }
    }
}
