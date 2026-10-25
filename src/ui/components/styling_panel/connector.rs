use crate::model::ConnectorVariant;
use crate::ui::components::styling_panel::utils::uniform_icon_seg;
use crate::ui::{icon, styles};
use leptos::*;
use std::rc::Rc;

fn variant_icon(variant: ConnectorVariant) -> View {
    match variant {
        ConnectorVariant::Straight => icon::connector_straight().into_view(),
        ConnectorVariant::Curved => icon::connector_curved().into_view(),
        ConnectorVariant::Direct => icon::connector_direct().into_view(),
    }
}

pub fn connector_section(
    is_connector: Signal<bool>,
    local_variant: RwSignal<ConnectorVariant>,
    set_variant: Rc<dyn Fn(ConnectorVariant)>,
) -> impl IntoView {
    let options: &[(ConnectorVariant, &str)] = &[
        (ConnectorVariant::Straight, "Straight"),
        (ConnectorVariant::Curved, "Curved"),
        (ConnectorVariant::Direct, "Direct"),
    ];

    view! {
        <div class:hidden=move || !is_connector.get()>
            <div>
                <div class=styles::SETTINGS_LABEL class:mb-1=move || true>
                    "Connector routing"
                </div>
                {uniform_icon_seg(
                    options,
                    Signal::derive(move || local_variant.get()),
                    set_variant,
                    variant_icon,
                )}
            </div>
        </div>
    }
}
