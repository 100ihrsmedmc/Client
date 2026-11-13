//! The touch-control layout editor (P2): a live preview of the touch controls
//! the player drags to move and taps to select for resizing and fading. It
//! edits the same [`TouchLayoutSettings`] the overlay and the input dispatch
//! read, so what the editor shows is what gameplay does; Save persists it and
//! Reset restores the built-in layout.
//!
//! The drag-to-move and per-control size/opacity model follows Horizon's
//! MIT-licensed touch layout editor; the code here is Pomme's own, built on
//! Pomme's menu toolkit and its existing touch layout.

use super::*;
use crate::app::input::touch::{TouchControl, TouchControls, TouchLayout};

/// The editor's in-progress state, kept on [`MainMenu`] so a drag survives the
/// per-frame rebuild.
#[derive(Default)]
pub struct TouchEditor {
    /// The control under a held press, following the pointer.
    dragging: Option<TouchControl>,
    /// The control whose size/opacity sliders show; the last one touched.
    selected: Option<TouchControl>,
    /// Pointer-to-centre offset at grab, so a control doesn't jump under the
    /// finger.
    grab: (f32, f32),
    /// Which slider (0 size, 1 opacity) is being dragged, so its capture
    /// survives leaving its bounds.
    active_slider: Option<u8>,
}

const BTN_W: f32 = 92.0;
const SLIDER_W: f32 = 210.0;
/// Vertical gap between the three sliders, in GUI units.
const SLIDER_PITCH: f32 = 24.0;
/// Highlight ring around the selected control, in GUI units beyond its radius.
const SELECT_GLOW: f32 = 6.0;
const COL_GLOW: [f32; 4] = [1.0, 0.9, 0.3, 0.35];
const COL_PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.55];

/// The y of the panel's first (global scale) slider, in pixels. A pure
/// function of the GUI scale and font size, so the builder and its tests agree.
fn sliders_top(gs: f32, fs: f32) -> f32 {
    let panel_y = 4.0 * gs;
    let title_y = panel_y + 4.0 * gs;
    let row_y = title_y + fs + 8.0 * gs;
    row_y + common::BTN_H * gs + 12.0 * gs
}

impl MainMenu {
    /// The touch-control layout editor. A full-screen preview of the controls,
    /// with a floating panel of Reset / Save / Done and, when a control is
    /// selected, its size and opacity sliders.
    pub(super) fn build_options_touch_layout(
        &mut self,
        sw: f32,
        sh: f32,
        input: &MenuInput,
        text_width_fn: common::TextWidthFn,
        key_bindings: &KeyBindings,
    ) -> MainMenuResult {
        if input.escape {
            self.set_screen(Screen::OptionsControls, key_bindings);
            return empty_result();
        }

        let gs = self.gui_scale(sw, sh);
        let fs = common::FONT_SIZE * gs;
        let btn_h = common::BTN_H * gs;
        let cx = sw / 2.0;
        let cursor = input.cursor;

        let mut elements = vec![MenuElement::BlurBackground];
        let mut any_hovered = false;

        // The live controls, drawn by the same code the HUD uses, so the
        // preview and gameplay agree.
        let layout = TouchLayout {
            width: sw,
            height: sh,
            gs,
            settings: self.touch_layout.clone(),
        };

        // The panel's geometry, so a press on it doesn't also grab a control.
        let btn_w = BTN_W * gs;
        let gap = 4.0 * gs;
        let row_w = btn_w * 3.0 + gap * 2.0;
        let slider_w = SLIDER_W * gs;
        let panel_w = row_w.max(slider_w) + 16.0 * gs;
        let panel_x = cx - panel_w / 2.0;
        let panel_y = 4.0 * gs;
        let title_y = panel_y + 4.0 * gs;
        let row_y = title_y + fs + 8.0 * gs;
        let sliders_y = sliders_top(gs, fs);
        let panel_bottom = sliders_y + 3.0 * SLIDER_PITCH * gs + 6.0 * gs;
        let panel_rect = [panel_x, panel_y, panel_w, panel_bottom - panel_y];

        // Drag a control: grab on press over one, follow the pointer, drop on
        // release. A press on the panel is not a grab.
        let pointer = (cursor.0, cursor.1);
        let on_panel = common::hit_test(cursor, panel_rect);
        if input.clicked && !on_panel {
            let grabbed = layout.control_at(glam::Vec2::new(pointer.0, pointer.1));
            self.touch_editor.dragging = grabbed;
            if let Some(control) = grabbed {
                self.touch_editor.selected = Some(control);
                let centre = layout.control_centre(control);
                self.touch_editor.grab = (centre.x - pointer.0, centre.y - pointer.1);
            }
        }
        if !input.mouse_held {
            self.touch_editor.dragging = None;
        }
        if let Some(control) = self.touch_editor.dragging {
            let grab = self.touch_editor.grab;
            self.touch_layout.set_centre(
                control,
                (pointer.0 + grab.0) / sw,
                (pointer.1 + grab.1) / sh,
            );
            self.touch_layout_dirty = true;
        }

        // A glow behind the selected/dragging control, then the controls.
        if let Some(control) = self.touch_editor.dragging.or(self.touch_editor.selected) {
            let centre = layout.control_centre(control);
            let radius = layout.control_radius(control) + SELECT_GLOW * gs;
            elements.push(MenuElement::Rect {
                x: centre.x - radius,
                y: centre.y - radius,
                w: 2.0 * radius,
                h: 2.0 * radius,
                corner_radius: radius,
                color: COL_GLOW,
            });
        }
        crate::ui::touch_controls::build_touch_controls(
            &mut elements,
            &layout,
            &TouchControls::default(),
        );

        // The floating panel: a translucent backing, the title, then widgets.
        elements.push(MenuElement::Rect {
            x: panel_rect[0],
            y: panel_rect[1],
            w: panel_rect[2],
            h: panel_rect[3],
            corner_radius: 4.0 * gs,
            color: COL_PANEL,
        });
        elements.push(MenuElement::Text {
            x: cx,
            y: title_y,
            text: "Touch Layout".into(),
            scale: fs,
            color: WHITE,
            centered: true,
        });

        self.focus_advance(input);
        let mut ctx = self.make_focus_ctx(input);

        let mut bx = cx - row_w / 2.0;
        let reset = push_button_f(
            &mut elements,
            &mut ctx,
            &mut any_hovered,
            cursor,
            input.clicked,
            bx,
            row_y,
            btn_w,
            btn_h,
            gs,
            "Reset",
            true,
        );
        bx += btn_w + gap;
        let save = push_button_f(
            &mut elements,
            &mut ctx,
            &mut any_hovered,
            cursor,
            input.clicked,
            bx,
            row_y,
            btn_w,
            btn_h,
            gs,
            "Save",
            true,
        );
        bx += btn_w + gap;
        let done = push_button_f(
            &mut elements,
            &mut ctx,
            &mut any_hovered,
            cursor,
            input.clicked,
            bx,
            row_y,
            btn_w,
            btn_h,
            gs,
            "Done",
            true,
        );

        if reset {
            self.touch_layout.reset_all();
            self.touch_layout_dirty = true;
            self.touch_editor.selected = None;
        }
        if save {
            self.touch_layout_dirty = true;
            self.flush_settings(key_bindings);
        }
        if done {
            self.flush_settings(key_bindings);
            self.set_screen(Screen::OptionsControls, key_bindings);
        }

        // The selected control's size and opacity sliders, and the global size.
        let placement = self
            .touch_editor
            .selected
            .map_or_else(Default::default, |c| self.touch_layout.placement(c));
        let control_label = self
            .touch_editor
            .selected
            .map_or("(tap a control)", TouchControl::name);
        let label_scroll = common::LabelScroll {
            text_width_fn,
            time_secs: self.created.elapsed().as_secs_f64(),
        };
        let sx = cx - slider_w / 2.0;
        let mut sy = sliders_y;

        // Global size: scales every control at once.
        let scale = self.touch_layout.scale_value();
        let scale_frac = (scale - crate::app::input::touch::SIZE_MIN)
            / (crate::app::input::touch::SIZE_MAX - crate::app::input::touch::SIZE_MIN);
        let scale_label = format!("All controls: {:.2}x", scale);
        let result = common::push_slider(
            &mut elements,
            cursor,
            input.clicked,
            input.mouse_held,
            sx,
            sy,
            slider_w,
            btn_h,
            gs,
            fs,
            &scale_label,
            scale_frac,
            true,
            false,
            true,
            self.touch_editor.active_slider == Some(2),
            &label_scroll,
        );
        any_hovered |= result.hovered;
        if result.dragging {
            self.touch_editor.active_slider = Some(2);
        }
        if let Some(v) = result.new_value {
            let scale = crate::app::input::touch::SIZE_MIN
                + v * (crate::app::input::touch::SIZE_MAX - crate::app::input::touch::SIZE_MIN);
            self.touch_layout.set_scale(scale);
            self.touch_layout_dirty = true;
        }
        if !input.mouse_held && self.touch_editor.active_slider == Some(2) {
            self.touch_editor.active_slider = None;
        }

        // Per-control size and opacity, for the selected control.
        sy += SLIDER_PITCH * gs;
        let size = placement.size_or_default();
        let size_frac = (size - crate::app::input::touch::SIZE_MIN)
            / (crate::app::input::touch::SIZE_MAX - crate::app::input::touch::SIZE_MIN);
        let size_label = format!("{control_label} size: {size:.2}x");
        let result = common::push_slider(
            &mut elements,
            cursor,
            input.clicked,
            input.mouse_held,
            sx,
            sy,
            slider_w,
            btn_h,
            gs,
            fs,
            &size_label,
            size_frac,
            true,
            false,
            true,
            self.touch_editor.active_slider == Some(0),
            &label_scroll,
        );
        any_hovered |= result.hovered;
        if result.dragging {
            self.touch_editor.active_slider = Some(0);
        }
        if let (Some(control), Some(v)) = (self.touch_editor.selected, result.new_value) {
            let size = crate::app::input::touch::SIZE_MIN
                + v * (crate::app::input::touch::SIZE_MAX - crate::app::input::touch::SIZE_MIN);
            self.touch_layout.set_size(control, size);
            self.touch_layout_dirty = true;
        }
        if !input.mouse_held && self.touch_editor.active_slider == Some(0) {
            self.touch_editor.active_slider = None;
        }

        sy += SLIDER_PITCH * gs;
        let opacity = placement.opacity_or_default();
        let opacity_label = format!("{control_label} opacity: {}%", (opacity * 100.0) as u32);
        let result = common::push_slider(
            &mut elements,
            cursor,
            input.clicked,
            input.mouse_held,
            sx,
            sy,
            slider_w,
            btn_h,
            gs,
            fs,
            &opacity_label,
            opacity,
            true,
            false,
            true,
            self.touch_editor.active_slider == Some(1),
            &label_scroll,
        );
        any_hovered |= result.hovered;
        if result.dragging {
            self.touch_editor.active_slider = Some(1);
        }
        if let (Some(control), Some(v)) = (self.touch_editor.selected, result.new_value) {
            self.touch_layout.set_opacity(control, v);
            self.touch_layout_dirty = true;
        }
        if !input.mouse_held && self.touch_editor.active_slider == Some(1) {
            self.touch_editor.active_slider = None;
        }

        elements.push(MenuElement::Text {
            x: cx,
            y: sh - 22.0 * gs,
            text: "Drag a control to move it. Tap one to resize or fade it.".into(),
            scale: fs,
            color: COL_DIM,
            centered: true,
        });

        self.finish_focus(&ctx);

        MainMenuResult {
            elements,
            action: MenuAction::None,
            cursor_pointer: any_hovered,
            clicked_button: ctx.fired,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::input::KeyBindings;
    use crate::renderer::pipelines::menu_overlay::MenuElement;

    const SW: f32 = 1024.0;
    const SH: f32 = 768.0;

    fn editor(dir: &str) -> (MainMenu, KeyBindings) {
        let mut kb = KeyBindings::default();
        let mut menu = test_menu(dir, &mut kb);
        menu.set_screen(Screen::OptionsTouchLayout, &kb);
        (menu, kb)
    }

    /// A cursor on a control's centre, at the editor's GUI scale.
    fn cursor_on(menu: &MainMenu, control: TouchControl) -> (f32, f32) {
        let gs = menu.gui_scale(SW, SH);
        let layout = TouchLayout {
            width: SW,
            height: SH,
            gs,
            settings: menu.touch_layout.clone(),
        };
        let c = layout.control_centre(control);
        (c.x, c.y)
    }

    fn press(
        menu: &mut MainMenu,
        kb: &KeyBindings,
        at: (f32, f32),
        clicked: bool,
        held: bool,
    ) -> MainMenuResult {
        menu.build(
            SW,
            SH,
            &MenuInput {
                cursor: at,
                clicked,
                mouse_held: held,
                ..MenuInput::default()
            },
            &mut kb.clone(),
            |_: &str, _: f32| 0.0,
        )
    }

    #[test]
    fn dragging_a_control_moves_it_and_marks_the_layout_dirty() {
        let dir = std::env::temp_dir().join(format!("pomme-editor-drag-{}", std::process::id()));
        let (mut menu, kb) = editor(dir.to_str().unwrap());
        assert!(!menu.touch_layout_dirty);

        // Press and hold on Jump's centre, drag the held pointer, then release.
        let start = cursor_on(&menu, TouchControl::Jump);
        let _ = press(&mut menu, &kb, start, true, true);
        assert_eq!(menu.touch_editor.dragging, Some(TouchControl::Jump));
        let _ = press(
            &mut menu,
            &kb,
            (start.0 - 100.0, start.1 - 80.0),
            false,
            true,
        );

        // The placement moved and the layout is dirty; releasing ends the drag.
        let placement = menu.touch_layout.placement(TouchControl::Jump);
        let (x, y) = placement.centre.expect("a drag saves a centre");
        assert!((x - (start.0 - 100.0) / SW).abs() < 1e-4, "x = {x}");
        assert!((y - (start.1 - 80.0) / SH).abs() < 1e-4, "y = {y}");
        assert!(menu.touch_layout_dirty);

        let _ = press(
            &mut menu,
            &kb,
            (start.0 - 100.0, start.1 - 80.0),
            false,
            false,
        );
        assert_eq!(menu.touch_editor.dragging, None, "release ends the drag");
    }

    #[test]
    fn a_press_on_the_panel_does_not_grab_a_control() {
        let dir = std::env::temp_dir().join(format!("pomme-editor-panel-{}", std::process::id()));
        let (mut menu, kb) = editor(dir.to_str().unwrap());

        // The title area of the panel is above every control.
        let _ = press(&mut menu, &kb, (SW / 2.0, 8.0), true, true);
        assert_eq!(menu.touch_editor.dragging, None);
    }

    #[test]
    fn tapping_a_control_selects_it_for_the_sliders() {
        let dir = std::env::temp_dir().join(format!("pomme-editor-select-{}", std::process::id()));
        let (mut menu, kb) = editor(dir.to_str().unwrap());

        let at = cursor_on(&menu, TouchControl::Inventory);
        let _ = press(&mut menu, &kb, at, true, true);
        assert_eq!(menu.touch_editor.selected, Some(TouchControl::Inventory));
    }

    #[test]
    fn reset_restores_the_built_in_layout() {
        let dir = std::env::temp_dir().join(format!("pomme-editor-reset-{}", std::process::id()));
        let (mut menu, kb) = editor(dir.to_str().unwrap());
        menu.touch_layout.set_centre(TouchControl::Pause, 0.1, 0.1);
        menu.touch_layout.set_scale(1.8);
        assert_ne!(menu.touch_layout, TouchLayoutSettings::default());

        // The Reset button is the leftmost of the three; click its centre.
        let gs = menu.gui_scale(SW, SH);
        let fs = common::FONT_SIZE * gs;
        let btn_w = BTN_W * gs;
        let gap = 4.0 * gs;
        let row_w = btn_w * 3.0 + gap * 2.0;
        let btn_h = common::BTN_H * gs;
        let panel_y = 4.0 * gs;
        let row_y = panel_y + 4.0 * gs + fs + 8.0 * gs;
        let x = SW / 2.0 - row_w / 2.0 + btn_w / 2.0;
        let _ = press(&mut menu, &kb, (x, row_y + btn_h / 2.0), true, true);

        assert_eq!(menu.touch_layout, TouchLayoutSettings::default());
        assert_eq!(menu.touch_editor.selected, None);
    }

    #[test]
    fn the_preview_draws_a_circle_per_control() {
        let dir = std::env::temp_dir().join(format!("pomme-editor-draw-{}", std::process::id()));
        let (mut menu, kb) = editor(dir.to_str().unwrap());
        let result = press(&mut menu, &kb, (-1.0, -1.0), false, false);

        // Five controls: the stick (base + knob) and four buttons, plus the
        // selection glow when one is selected (none here).
        let circles = result
            .elements
            .iter()
            .filter(
                |e| matches!(e, MenuElement::Rect { corner_radius, .. } if *corner_radius > 0.0),
            )
            .count();
        assert!(circles >= 6, "expected the controls to draw, got {circles}");
    }
}
