use iced::{
    button, scrollable, text_input, Scrollable, Container, container,
    Color,
    Alignment, Button, Column, Element, Length, Row, Text, TextInput
};
use iced::alignment::{Horizontal, Vertical};
use sp_domain::{SPPath, SPValue, SPState, Operation, Intention};
use sp_formal::CompiledModel;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::Message;

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
    pub reset_button_state: button::State,
}

#[derive(Debug, Clone)]
pub struct IntentionInfo {
    pub i: Intention,
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
                Button::new(&mut self.reset_button_state, Text::new("Reset").size(12))
                    .padding(10)
                    .on_press(Message::ResetOperation(self.op.path.clone())),
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
            .push(
                Button::new(&mut self.reset_button_state, Text::new("Reset"))
                    .padding(10)
                    .on_press(Message::ResetOperation(self.i.path.clone())),
            )
            .into()
    }
}

impl SPModelInfo {
    pub(crate) fn from(compiled_model: CompiledModel) -> SPModelInfo {
        let operations = compiled_model
            .model
            .operations
            .iter()
            .map(|o| OperationInfo {
                op: o.clone(),
                reset_button_state: button::State::new(),
            })
            .collect();
        let intentions = compiled_model
            .model
            .intentions
            .iter()
            .map(|i| IntentionInfo {
                i: i.clone(),
                reset_button_state: button::State::new(),
            })
            .collect();

        SPModelInfo {
            state: vec![],
            operations,
            intentions,
            buffers_locations: (0..9).map(|_| BufferLocation::default()).collect(),
            estimated_locations: (0..9).map(|_| BufferLocation::default()).collect(),
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

    pub(crate) fn view_demo_goal(&mut self) -> Element<Message> {
        let col = Column::new().spacing(10);
        let col = col.push(Text::new("Create an order").size(20));
        // grid of 9 buttons to make an "order"
        let grid: Element<Message> =
            self.estimated_locations
            .chunks_mut(3)
            .enumerate()
            .fold(Row::new().spacing(10), |row, (y, bl) | {
                let r = bl.iter_mut().enumerate()
                    .fold(Column::new().spacing(10), |col, (x, b)| {
                        let text = format!("{},{}: {}", x, y,
                                           if b.value { "cylinder" } else
                                           { "empty" } );
                        let button = Button::new(&mut b.button, Text::new(text))
                            .on_press(Message::Empty);
                        col.push(button)
                    });
                row.push(r)
            })
            .into();

        let col = col.push(grid);

        let button = Button::new(&mut self.update_button, Text::new("Update state"))
            .padding(10)
            .style(crate::style::Button::Primary);
        let col = col.push(button);


        ///////////////

        let col = col.push(Text::new("Create an order").size(20));
        // grid of 9 buttons to make an "order"
        let grid: Element<Message> =
            self.buffers_locations
            .chunks_mut(3)
            .enumerate()
            .fold(Row::new().spacing(10), |row, (y, bl) | {
                let r = bl.iter_mut().enumerate()
                    .fold(Column::new().spacing(10), |col, (x, b)| {
                        let text = format!("{},{}: {}", x, y, "empty");
                        col.push(Button::new(&mut b.button, Text::new(text)))
                    });
                row.push(r)
            })
            .into();

        let col = col.push(grid);

        let button = Button::new(&mut self.order_button, Text::new("Make order"))
            .padding(10)
            .style(crate::style::Button::Primary)
            .on_press(Message::SetEstimatedCylinders);
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
              .push(Text::new(path.to_string()).size(10).color([0.3, 0.3, 0.3])))
        .push(Column::new()
              .width(Length::FillPortion(1))
              .push(Text::new(&value).size(10).color([0.2, 0.2, 0.2])))
        .push(
            TextInput::new(
                text_state,
                "",
                new_value,
                move |new_value| Message::StateValueEdit(path.clone(), new_value)
            ))
        .into()
}
