use iced::{
    button, scrollable, text_input, Scrollable, Container, container,
    Color,
    Alignment, Button, Column, Element, Length, Row, Text, TextInput
};
use iced::alignment::{Horizontal, Vertical};
use sp_domain::*;
use sp_formal::CompiledModel;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::{Message, BufferLocationType};

// SIDExSIDE buffer view for goal generation.
static NUM_BUFFERS_SIDE: usize = 2;

#[derive(Debug, Copy, Clone)]
pub enum NotificationType {
    Happy,
    Neutral,
    Sad,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub type_: NotificationType,
    pub close_button_state: button::State,
}

impl Notification {
    pub fn new(message: String, type_: NotificationType) -> Self {
        Notification {
            message,
            type_,
            close_button_state: button::State::new(),
        }
    }

    pub fn view(&mut self) -> Container<Message> {
        let close_button = Button::new(&mut self.close_button_state, Text::new("X"))
            .on_press(Message::ClearNotification)
            .style(NotificationStyle {
                type_: self.type_,
            });

        let contents_row = Row::new()
            .push(
                Container::new(Text::new(&self.message).size(18))
                    .width(Length::Fill),
            )
            .push(close_button)
            .align_items(Alignment::Center);

        Container::new(
            Container::new(contents_row)
                .width(Length::Fill)
                .padding(20)
                .style(NotificationStyle {
                    type_: self.type_,
                }),
        )
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .width(Length::Fill)
        .padding(1)
    }
}

struct NotificationStyle {
    type_: NotificationType,
}

impl container::StyleSheet for NotificationStyle {
    fn style(&self) -> container::Style {
        match self.type_ {
            NotificationType::Happy => HappyNotificationStyle.style(),
            NotificationType::Neutral => NeutralNotificationStyle.style(),
            NotificationType::Sad => SadNotificationStyle.style(),
        }
    }
}

impl button::StyleSheet for NotificationStyle {
    fn active(&self) -> button::Style {
        match self.type_ {
            NotificationType::Happy => HappyNotificationStyle.active(),
            NotificationType::Neutral => NeutralNotificationStyle.active(),
            NotificationType::Sad => SadNotificationStyle.active(),
        }
    }
}

struct HappyNotificationStyle;

impl container::StyleSheet for HappyNotificationStyle {
    fn style(&self) -> container::Style {
        container::Style {
            text_color: Some(Color::from_rgb8(0, 0, 0)),
            background: Some(Color::from_rgb8(200, 255, 200).into()),
            border_radius: 3.0,
            border_width: 1.0,
            border_color: Color::from_rgb8(50, 200, 50),
        }
    }
}

impl button::StyleSheet for HappyNotificationStyle {
    fn active(&self) -> button::Style {
        button::Style {
            shadow_offset: Default::default(),
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::from_rgb8(29, 54, 39),
            text_color: Color::from_rgb8(0, 0, 0),
            ..button::Style::default()
        }
    }
}

struct NeutralNotificationStyle;

impl container::StyleSheet for NeutralNotificationStyle {
    fn style(&self) -> container::Style {
        container::Style {
            text_color: Some(Color::from_rgb8(0, 0, 0)),
            background: Some(Color::from_rgb8(159, 169, 184).into()),
            border_radius: 3.0,
            border_width: 1.0,
            border_color: Color::from_rgb8(36, 47, 61),
        }
    }
}

impl button::StyleSheet for NeutralNotificationStyle {
    fn active(&self) -> button::Style {
        button::Style {
            shadow_offset: Default::default(),
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::from_rgb8(36, 47, 61),
            text_color: Color::from_rgb8(0, 0, 0),
            ..button::Style::default()
        }
    }
}

struct SadNotificationStyle;

impl container::StyleSheet for SadNotificationStyle {
    fn style(&self) -> container::Style {
        container::Style {
            text_color: Some(Color::from_rgb8(0, 0, 0)),
            background: Some(Color::from_rgb8(255, 200, 200).into()),
            border_radius: 3.0,
            border_width: 1.0,
            border_color: Color::from_rgb8(200, 50, 50),
        }
    }
}

impl button::StyleSheet for SadNotificationStyle {
    fn active(&self) -> button::Style {
        button::Style {
            shadow_offset: Default::default(),
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::from_rgb8(84, 49, 49),
            text_color: Color::from_rgb8(0, 0, 0),
            ..button::Style::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub op: Operation,
    pub start_button_state: button::State,
    pub reset_button_state: button::State,
}

#[derive(Debug, Clone)]
pub struct IntentionInfo {
    pub i: Intention,
    pub stop_button_state: button::State,
    pub start_button_state: button::State,
    pub reset_button_state: button::State,
}

#[derive(Debug, Clone, Default)]
pub struct StateInfo {
    pub path: SPPath,
    pub value: SPValue,
    pub new_value: String,
    pub new_value_state: text_input::State,
}

#[derive(Debug, Clone, Default)]
pub struct BufferLocation {
    pub value: bool,
    pub button: button::State,
}

#[derive(Debug, Clone, Default)]
pub struct SPModelInfo {
    pub compiled_model: CompiledModel,
    pub state: Vec<StateInfo>,
    pub operations: Vec<OperationInfo>,
    pub intentions: Vec<IntentionInfo>,
    pub buffers_locations: Vec<BufferLocation>,
    pub estimated_locations: Vec<BufferLocation>,
    pub order_button: button::State,
    pub update_button: button::State,
}

impl OperationInfo {
    pub(crate) fn view(&mut self, state_value: &str) -> Element<Message> {
        Row::new()
            .align_items(Alignment::Center)
            .spacing(20)
            .push(
                Text::new(self.op.path().leaf())
                    .size(30)
                    .width(Length::Fill),
            )
            .push(
                Text::new(self.op.path().to_string())
                    .size(10)
                    .color([0.5, 0.5, 0.5]),
            )
            .push(Text::new(state_value).size(20).color([0.2, 0.2, 0.2]))
            .push(
                Button::new(&mut self.start_button_state, Text::new("Force start").size(10))
                    .padding(10)
                    .on_press(Message::ResetOperation(self.op.path.clone(), "e".to_spvalue())),
            )
            .push(
                Button::new(&mut self.reset_button_state, Text::new("Reset").size(10))
                    .padding(10)
                    .on_press(Message::ResetOperation(self.op.path.clone(), "i".to_spvalue())),
            )
            .into()
    }
}

impl IntentionInfo {
    pub(crate) fn view(&mut self, state_value: &str) -> Element<Message> {
        Row::new()
            .align_items(Alignment::Center)
            .spacing(20)
            .push(Text::new(self.i.path().leaf()).size(30).width(Length::Fill))
            .push(
                Text::new(self.i.path().to_string())
                    .size(10)
                    .color([0.5, 0.5, 0.5]),
            )
            .push(Text::new(state_value).size(20).color([0.2, 0.2, 0.2]))
            .push(Button::new(&mut self.stop_button_state, Text::new("Stop").size(10))
                              .padding(10)
                              .on_press(Message::ResetOperation(self.i.path.clone(), "X".to_spvalue())))
            .push(Button::new(&mut self.start_button_state, Text::new("Force start").size(10))
                              .padding(10)
                              .on_press(Message::ResetOperation(self.i.path.clone(), "e".to_spvalue())))
            .push(Button::new(&mut self.reset_button_state, Text::new("Reset").size(10))
                              .padding(10)
                              .on_press(Message::ResetOperation(self.i.path.clone(), "i".to_spvalue())))
            .into()
    }
}

impl SPModelInfo {
    pub(crate) fn get_spstate(&self) -> SPState {
        let state: Vec<_> = self.state.iter().map(|si| (si.path.clone(), si.value.clone())).collect();
        SPState::new_from_values(&state)
    }

    pub(crate) fn from(compiled_model: CompiledModel) -> SPModelInfo {
        let operations = compiled_model
            .model
            .operations
            .iter()
            .map(|o| OperationInfo {
                op: o.clone(),
                start_button_state: button::State::new(),
                reset_button_state: button::State::new(),
            })
            .collect();
        let intentions = compiled_model
            .model
            .intentions
            .iter()
            .map(|i| IntentionInfo {
                i: i.clone(),
                stop_button_state: button::State::new(),
                start_button_state: button::State::new(),
                reset_button_state: button::State::new(),
            })
            .collect();

        let num_buffers = NUM_BUFFERS_SIDE * NUM_BUFFERS_SIDE;
        SPModelInfo {
            compiled_model,
            state: vec![],
            operations,
            intentions,
            buffers_locations: (0..num_buffers).map(|_| BufferLocation::default()).collect(),
            estimated_locations: (0..num_buffers).map(|_| BufferLocation::default()).collect(),
            order_button: button::State::default(),
            update_button: button::State::default(),
        }
    }

    pub(crate) fn view_ops(&mut self) -> Element<Message> {
        self.operations
            .iter_mut()
            .fold(Column::new().spacing(10), |col, o| {
                let state_value = self.state.iter()
                    .find(|s| s.path == o.op.path)
                    .map(|s| s.value.to_string()).unwrap_or("[no state]".into());
                col.push(o.view(&state_value))
            })
            .into()
    }

    pub(crate) fn view_ints(&mut self) -> Element<Message> {
        self.intentions
            .iter_mut()
            .fold(Column::new().spacing(10), |col, i| {
                let state_value = self.state.iter()
                    .find(|s| s.path == i.i.path)
                    .map(|s| s.value.to_string()).unwrap_or("[no state]".into());
                col.push(i.view(&state_value))
            })
            .into()
    }

    pub(crate) fn view_tplan(&mut self) -> Element<Message> {
        // temp hack to get transitions, move this out later.
        let model = &self.compiled_model.model;
        let mut transitions: Vec<Transition> = model.resources
            .iter()
            .flat_map(|r| r.transitions.clone())
            .collect();

        let global_transitions = model.global_transitions.clone();
        transitions.extend(global_transitions);
        // end temp hack

        let goal_p = SPPath::from_slice(&["runner", "transition_goal"]);
        let goal_str = self.state.iter()
            .find(|si| si.path == goal_p)
            .map(|si| si.value.to_string()).unwrap_or("no goal".to_string());

        let plan_idx = self.state.iter().find_map(|si| {
            if si.path == SPPath::from_slice(&["runner", "plans", "0"]) {
                if let SPValue::Int32(idx) = &si.value {
                    return Some(*idx);
                }
            }
            None
        }).unwrap_or(0);

        let p_path = SPPath::from_slice(&["runner", "transition_plan"]);
        let paths = self.state.iter().find_map(|si| {
            if si.path == p_path {
                if let SPValue::Array(SPValueType::Path, v) = &si.value {
                    Some(v.iter().map(|e| e.to_string()).collect())
                } else {
                    None
                }
            } else {
                None
            }
        }).unwrap_or(vec![]);

        let mut idx = 0;
        let mut path_info = vec![];
        for p in paths {
            let trans = transitions.iter().find(|t| t.path().to_string() == p);
            if let Some(trans) = &trans {
                if trans.type_ == TransitionType::Controlled {
                    idx+=1;
                }
            }
            path_info.push((p, trans, idx));
        }

        let s = self.get_spstate();

        let plan_cols: Element<Message> = path_info
            .iter()
            .fold(Column::new().spacing(10), |col, (path, trans, idx)| {
                let mut guard = false;
                let mut runner_guard = false;
                let mut show_guards = false;
                let color = match trans {
                    Some(trans) if trans.type_ == TransitionType::Controlled => {
                        if idx > &plan_idx {
                            guard = trans.guard.eval(&s);
                            runner_guard = trans.runner_guard.eval(&s);
                            show_guards = true;
                            [0.3, 0.3, 0.3] // later in plan
                        } else if idx == &plan_idx {
                            [0.0, 0.0, 0.5] // just started
                        } else {
                            [0.0, 0.5, 0.0] // already done
                        }
                    },
                    Some(_) => [0.5, 0.5, 0.5], // effects are shaded
                    None => [0.8, 0.8, 0.8], // trans does not exist
                };

                let guard_str = if show_guards {
                    format!("g: {} / rg: {}", guard, runner_guard)
                } else {
                    String::new()
                };
                col.push(
                    Row::new()
                        .push(Column::new().width(Length::FillPortion(3))
                              .push(Text::new(path).color(color)))
                        .push(Column::new().width(Length::FillPortion(1))
                              .push(Text::new(guard_str).color(color))))
            })
            .into();

        Column::new()
            .spacing(5)
            .push(Text::new(goal_str).size(30))
            .push(Column::new().push(plan_cols)).into()
    }

    pub(crate) fn view_oplan(&mut self) -> Element<Message> {
        let goal_p = SPPath::from_slice(&["runner", "operation_goal"]);
        let goal_str = self.state.iter()
            .find(|si| si.path == goal_p)
            .map(|si| si.value.to_string()).unwrap_or("no goal".to_string());

        let p_path = SPPath::from_slice(&["runner", "operation_plan"]);
        let paths = self.state.iter().find_map(|si| {
            if si.path == p_path {
                if let SPValue::Array(SPValueType::Path, v) = &si.value {
                    Some(v.iter().map(|e| e.to_string()).collect())
                } else {
                    None
                }
            } else {
                None
            }
        }).unwrap_or(vec![]);
        let plan_cols: Element<Message> = paths
            .iter()
            .fold(Column::new().spacing(10), |col, path| {
                col.push(Text::new(path))
            })
            .into();

        Column::new()
            .height(Length::Fill)
            .spacing(20)
            .push(Text::new(goal_str).size(30))
            .push(Column::new().push(plan_cols)).into()
    }

    pub(crate) fn view_demo_goal(&mut self) -> Element<Message> {
        let col = Column::new().spacing(10);
        let col = col.push(Text::new("Set system state").size(20));

        let grid: Element<Message> =
            self.estimated_locations
            .chunks_mut(NUM_BUFFERS_SIDE)
            .enumerate()
            .fold(Column::new().spacing(10), |row, (y, bl) | {
                let r = bl.iter_mut().enumerate()
                    .fold(Row::new().spacing(10), |col, (x, b)| {
                        let val_str = if b.value { "cylinder" } else { "empty" };
                        let text = format!("{},{}: {}", x, y, val_str);
                        let message = Message::BufferButton(
                            BufferLocationType::Estimated,
                            y*NUM_BUFFERS_SIDE+x, !b.value);
                        let button = Button::new(&mut b.button, Text::new(text))
                            .on_press(message);
                        col.push(button)
                    });
                row.push(r)
            })
            .into();

        let col = col.push(grid);

        let button = Button::new(&mut self.update_button, Text::new("Update state"))
            .padding(10)
            .style(crate::style::Button::Primary)
            .on_press(Message::SetEstimatedCylinders);
        let col = col.push(button);


        ///////////////

        let col = col.push(Text::new("Create an order").size(20));

        let grid: Element<Message> =
            self.buffers_locations
            .chunks_mut(NUM_BUFFERS_SIDE)
            .enumerate()
            .fold(Column::new().spacing(10), |row, (y, bl) | {
                let r = bl.iter_mut().enumerate()
                    .fold(Row::new().spacing(10), |col, (x, b)| {
                        let val_str = if b.value { "cylinder" } else { "empty" };
                        let text = format!("{},{}: {}", x, y, val_str);
                        let message = Message::BufferButton(
                            BufferLocationType::Goal,
                            y*NUM_BUFFERS_SIDE+x, !b.value);
                        col.push(Button::new(&mut b.button, Text::new(text))
                                 .on_press(message))
                    });
                row.push(r)
            })
            .into();

        let col = col.push(grid);

        let button = Button::new(&mut self.order_button, Text::new("Make order"))
            .padding(10)
            .style(crate::style::Button::Primary)
            .on_press(Message::SendGoalCylinders);
        let col = col.push(button);

        col.into()
    }

    pub(crate) fn view_state<'a>(&'a mut self,
                                 filter: &'a str,
                                 scroll_state: &'a mut scrollable::State) -> Element<Message> {
        let matcher = SkimMatcherV2::default().ignore_case();
        self.state.sort_by(|a,b| a.path.cmp(&b.path));
        let state: Element<Message> = self.state.iter_mut()
            .filter(|si| matcher.fuzzy_match(&si.path.to_string(), filter).is_some())
            .fold(Column::new().spacing(5),
                  |col, si| col.push(
                      view_state_row(si.path.clone(),
                                     si.value.to_string(),
                                     &si.new_value,
                                     &mut si.new_value_state))).into();

        Scrollable::new(scroll_state)
                    .push(Container::new(state)).into()
    }
}


pub(crate) fn view_state_row<'a>(path: SPPath, value: String,
                                 new_value: &str, text_state: &'a mut text_input::State) -> Element<'a, Message> {
    Row::new()
        .spacing(20)
        .push(Column::new()
              .width(Length::FillPortion(3))
              .push(Text::new(path.to_string()).size(16).color([0.3, 0.3, 0.3])))
        .push(Column::new()
              .width(Length::FillPortion(1))
              .push(Text::new(&value).size(16).height(Length::Units(30)).color([0.2, 0.2, 0.2])))
        .push(
            TextInput::new(
                text_state,
                "",
                new_value,
                move |new_value| Message::StateValueEdit(path.clone(), new_value)
            ))
        .into()
}
