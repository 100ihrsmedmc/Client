//! Draws the touch controls (`app::input::touch`) over the HUD: the move
//! stick, resting bottom-left until a press places it, and the buttons.

use glam::Vec2;

use crate::app::input::touch::{TouchButton, TouchControls, TouchLayout};
use crate::renderer::pipelines::menu_overlay::MenuElement;
use crate::ui::common::{FONT_SIZE, WHITE};

const BASE: [f32; 4] = [0.0, 0.0, 0.0, 0.35];
const KNOB: [f32; 4] = [1.0, 1.0, 1.0, 0.45];

pub fn build_touch_controls(
    elements: &mut Vec<MenuElement>,
    layout: &TouchLayout,
    touch: &TouchControls,
) {
    let gs = layout.gs;
    // The stick's radius and opacity come from the layout, so a resized or
    // faded stick draws the size and alpha the input dispatch hit-tests.
    let radius = layout.stick_radius();
    let base = touch.stick_origin().unwrap_or_else(|| layout.stick_home());
    push_circle(elements, base, radius, fade(BASE, layout.stick_opacity()));
    let deflection = touch.stick().map_or(Vec2::ZERO, |v| Vec2::new(v.x, -v.y));
    push_circle(
        elements,
        base + deflection * radius,
        radius * 0.45,
        fade(KNOB, layout.stick_opacity()),
    );

    for button in TouchButton::ALL {
        let centre = layout.button_centre(button);
        let opacity = layout.button_opacity(button);
        push_circle(
            elements,
            centre,
            layout.button_radius(button),
            fade(BASE, opacity),
        );
        elements.push(MenuElement::Text {
            x: centre.x,
            y: centre.y - FONT_SIZE * gs / 2.0,
            text: button.label().to_owned(),
            scale: FONT_SIZE * gs,
            color: fade(WHITE, opacity),
            centered: true,
        });
    }
}

/// A colour with its alpha multiplied by `opacity`, for a faded control.
fn fade(color: [f32; 4], opacity: f32) -> [f32; 4] {
    [color[0], color[1], color[2], color[3] * opacity]
}

fn push_circle(elements: &mut Vec<MenuElement>, centre: Vec2, radius: f32, color: [f32; 4]) {
    elements.push(MenuElement::Rect {
        x: centre.x - radius,
        y: centre.y - radius,
        w: 2.0 * radius,
        h: 2.0 * radius,
        corner_radius: radius,
        color,
    });
}
