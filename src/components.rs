use iced::{
    button, Alignment, Button, Column, Element, Length, Row, Text,
};
use sp_domain::{SPState, Operation, Intention};
use sp_formal::CompiledModel;
use crate::Message;

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

#[derive(Debug, Clone)]
pub struct SPModelInfo {
    pub operations: Vec<OperationInfo>,
    pub intentions: Vec<IntentionInfo>,
}

impl OperationInfo {
    pub(crate) fn view(&mut self, state: &SPState) -> Element<Message> {
        let op_state = state
            .sp_value_from_path(&self.op.path())
            .expect("value")
            .to_string();

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
            .push(Text::new(op_state).size(20).color([0.2, 0.2, 0.2]))
            .push(
                Button::new(&mut self.reset_button_state, Text::new("Reset").size(12))
                    .padding(10)
                    .on_press(Message::ResetOperation(self.op.path.clone())),
            )
            .into()
    }
}

impl IntentionInfo {
    pub(crate) fn view(&mut self, state: &SPState) -> Element<Message> {
        let i_state = state
            .sp_value_from_path(&self.i.path())
            .expect("value")
            .to_string();

        Row::new()
            .align_items(Alignment::Center)
            .spacing(20)
            .push(Text::new(self.i.path().leaf()).size(30).width(Length::Fill))
            .push(
                Text::new(self.i.path().to_string())
                    .size(10)
                    .color([0.5, 0.5, 0.5]),
            )
            .push(Text::new(i_state).size(20).color([0.2, 0.2, 0.2]))
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
            operations,
            intentions,
        }
    }

    pub(crate) fn view(&mut self, state: &SPState, iv: bool) -> Element<Message> {
        let list_view: Element<_> = if iv {
            self.intentions
                .iter_mut()
                .fold(Column::new().spacing(10), |col, i| col.push(i.view(state)))
                .into()
        } else {
            self.operations
                .iter_mut()
                .fold(Column::new().spacing(10), |col, o| col.push(o.view(state)))
                .into()
        };

        Row::new()
            .spacing(20)
            .align_items(Alignment::Center)
            .push(Column::new().spacing(20).push(list_view))
            .into()
    }
}
