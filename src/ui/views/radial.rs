//! Radial section: text-free surface first, geometry + assignments below.

use std::cell::RefCell;
use std::sync::Arc;

use iced::widget::{
    button, column, container, keyed_column, pick_list, row, slider, text, toggler,
};
use iced::{Element, Length};

use crate::core::actions::WindowAction;
use crate::core::radial::GEOMETRY;
use crate::ui::app::{Message, Section, State};
use crate::ui::widgets::radial_canvas::{self, RadialCanvas};

struct TargetChoiceKey {
    id: Arc<str>,
    modifiers: u32,
    vk: u32,
}

struct TargetChoiceCache {
    keybinds: Vec<TargetChoiceKey>,
    choices: Arc<[Arc<str>]>,
}

thread_local! {
    static TARGET_CHOICE_CACHE: RefCell<Option<TargetChoiceCache>> = const { RefCell::new(None) };
}

fn action_choices() -> &'static Arc<[Arc<str>]> {
    static CHOICES: std::sync::OnceLock<Arc<[Arc<str>]>> = std::sync::OnceLock::new();

    CHOICES.get_or_init(|| {
        let mut choices: Vec<Arc<str>> = vec![Arc::from("No action")];
        choices.extend(
            WindowAction::ALL
                .iter()
                .map(|action| Arc::from(format!("Action: {}", action.display_name()))),
        );
        choices.into()
    })
}

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;
    let canvas = RadialCanvas::from_settings(settings);

    let choices = target_choices(settings);
    let mut assignments =
        keyed_column([(0_u8, text("Wedge assignments").size(15).into())]).spacing(6);
    for (index, slot) in GEOMETRY.iter().enumerate() {
        let target = settings.radial_slots.get(index);
        let name = target_choice(settings, target);
        assignments = assignments.push(
            (index + 1) as u8,
            row![
                text(format!("{} ", slot.label)).size(13),
                pick_list(choices.clone(), Some(name), {
                    move |choice: Arc<str>| {
                        Message::SetRadialTarget(Some(index), choice.to_string())
                    }
                }),
                toggler(target.is_some_and(|target| target.cycle_enabled))
                    .on_toggle(move |_| Message::ToggleRadialCycle(Some(index))),
                button("Clear").on_press(Message::ClearWedge(index)),
            ]
            .spacing(10),
        );
    }

    let center = target_choice(settings, Some(&settings.center_target));
    assignments = assignments.push(
        (GEOMETRY.len() + 1) as u8,
        row![
            text("Center").size(13),
            pick_list(choices, Some(center), |choice: Arc<str>| {
                Message::SetRadialTarget(None, choice.to_string())
            },),
            toggler(settings.center_target.cycle_enabled)
                .on_toggle(|_| Message::ToggleRadialCycle(None)),
        ]
        .spacing(10),
    );

    let content = column![
        text("Radial menu").size(20),
        text("The overlay itself never shows text — labels here are settings-only.").size(13),
        radial_canvas::view(canvas, 260.0),
        row![
            text("Enabled").size(14),
            toggler(settings.radial_enabled).on_toggle(Message::SetRadialEnabled),
            text("Cursor follows direction").size(14),
            toggler(settings.cursor_interaction_enabled).on_toggle(Message::SetCursorInteraction),
        ]
        .spacing(12),
        text(format!(
            "Outer radius: {:.0} px",
            settings.radial_outer_radius
        ))
        .size(14),
        slider(
            64.0..=140.0,
            settings.radial_outer_radius as f32,
            Message::SetOuterRadius
        ),
        text(format!(
            "Inner radius: {:.0} px",
            settings.radial_inner_radius
        ))
        .size(14),
        slider(
            24.0..=132.0,
            settings.radial_inner_radius as f32,
            Message::SetInnerRadius
        ),
        assignments,
        row![button("Reset section").on_press(Message::ResetSection(Section::Radial)),],
        text(&state.status).size(12),
    ]
    .spacing(12)
    .padding(16);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn target_choices(settings: &StateSettings) -> Arc<[Arc<str>]> {
    if settings.keybinds.is_empty() {
        return Arc::clone(action_choices());
    }

    TARGET_CHOICE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let cache_matches = cache.as_ref().is_some_and(|cached| {
            cached.keybinds.len() == settings.keybinds.len()
                && cached
                    .keybinds
                    .iter()
                    .zip(&settings.keybinds)
                    .all(|(cached, bind)| {
                        cached.id.as_ref() == bind.id
                            && cached.modifiers == bind.modifiers
                            && cached.vk == bind.vk
                    })
        });
        if cache_matches {
            return Arc::clone(&cache.as_ref().expect("matched cache").choices);
        }

        let mut choices = action_choices().iter().cloned().collect::<Vec<_>>();
        choices.extend(settings.keybinds.iter().map(|bind| {
            Arc::from(format!(
                "Keybind: {} ({})",
                bind.id,
                crate::core::hotkey::hotkey_name(
                    bind.modifiers,
                    bind.vk,
                    crate::core::hotkey::TriggerModifierSide::Any,
                )
            ))
        }));
        let choices: Arc<[Arc<str>]> = choices.into();
        let keybinds = settings
            .keybinds
            .iter()
            .map(|bind| TargetChoiceKey {
                id: Arc::from(bind.id.as_str()),
                modifiers: bind.modifiers,
                vk: bind.vk,
            })
            .collect();
        *cache = Some(TargetChoiceCache {
            keybinds,
            choices: choices.clone(),
        });
        choices
    })
}

type StateSettings = crate::settings::AppSettings;

fn target_choice(
    settings: &StateSettings,
    target: Option<&crate::core::radial_targets::RadialTargetSettings>,
) -> Arc<str> {
    let Some(target) = target else {
        return Arc::from("No action");
    };
    match target.kind {
        crate::core::radial_targets::RadialTargetKind::None => Arc::from("No action"),
        crate::core::radial_targets::RadialTargetKind::Action => {
            Arc::from(format!("Action: {}", target.action.display_name()))
        }
        crate::core::radial_targets::RadialTargetKind::Keybind => settings
            .keybinds
            .iter()
            .find(|bind| bind.id == target.keybind_id)
            .map(|bind| {
                Arc::from(format!(
                    "Keybind: {} ({})",
                    bind.id,
                    crate::core::hotkey::hotkey_name(
                        bind.modifiers,
                        bind.vk,
                        crate::core::hotkey::TriggerModifierSide::Any,
                    )
                ))
            })
            .unwrap_or_else(|| Arc::from("No action")),
    }
}
