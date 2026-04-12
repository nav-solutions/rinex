use crate::header::Header;

use qc_traits::{html, Markup, QcHtmlReporting};

#[cfg(feature = "qc")]
impl QcHtmlReporting for Header {
    fn render(&self) -> Markup {
        html! {
            tr {
                th { "Antenna" }
                @if let Some(antenna) = &self.rcvr_antenna {
                    td { (antenna.render()) }
                } @else {
                    td { "No information" }
                }
            }
            tr {
                th { "Receiver" }
                @ if let Some(rcvr) = &self.rcvr {
                    td { (rcvr.render()) }
                } else {
                    td { "No information" }
                }
            }
        }
    }
}
