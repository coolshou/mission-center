use std::sync::Arc;

use dbus::{arg::*, strings::*};

pub struct ArcStrVec(Vec<Arc<str>>);

impl From<Vec<Arc<str>>> for ArcStrVec {
    fn from(value: Vec<Arc<str>>) -> Self {
        Self(value)
    }
}

impl From<ArcStrVec> for Vec<Arc<str>> {
    fn from(value: ArcStrVec) -> Self {
        value.0
    }
}

impl Arg for ArcStrVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("as")
    }
}

impl<'a> Get<'a> for ArcStrVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<Arc<str>>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<Arc<str>>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for s in arr {
                        if let Some(s) = s.as_str() {
                            this.push(Arc::from(s));
                        }
                    }
                }
            },
        }

        Some(this.into())
    }
}
