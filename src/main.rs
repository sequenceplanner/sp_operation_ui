use futures::future;
use futures::stream::StreamExt;
use iced::{
    window, button, scrollable, text_input, Alignment, Application, Button, Column, Command, Container, Element, Length, Row, Settings, Subscription, Text
};
use iced_native::subscription;
use r2r;
use serde::Deserialize;
use sp_domain::*;
use sp_formal::CompiledModel;
use std::sync::{Arc, Mutex};

mod components;
use components::*;

pub fn main() -> iced::Result {
    SPOpViewer::run(
        Settings {
            antialiasing: true,
            window: window::Settings {
                position: window::Position::Centered,
                size: (600, 700),
                ..window::Settings::default()
            },
            ..Settings::default()
        })
}

struct SPOpViewer {
    // ros communication stuff
    get_model_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    set_state_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    new_state_receiver: Mutex<Option<tokio::sync::mpsc::Receiver<SPState>>>,

    // our ui state
    ui_state: SPOpViewerState,

    // other state...
    notification: Option<Notification>,
}

#[derive(Debug, Clone)]
enum View {
    OperationView,
    IntentionView,
    StateView,
    DemoGoalView,
}

#[derive(Debug, Clone)]
enum SPOpViewerState {
    Loading,
    Loaded {
        model_info: SPModelInfo,
        current_view: View,
        scroll: scrollable::State,
        footer: Footer,
    },
    Errored {
        get_model_button: button::State,
    },
}

#[derive(Debug, Clone, Default)]
struct Footer {
    op_view_button: button::State,
    int_view_button: button::State,
    state_view_button: button::State,
    get_model_button: button::State,
    make_goal_button: button::State,
}

impl Footer {
    fn view(&mut self) -> Element<Message> {
        Row::new()
            .spacing(20)
            .push(button(&mut self.op_view_button, "Operations").on_press(Message::OperationView))
            .push(button(&mut self.int_view_button, "Intentions").on_press(Message::IntentionView))
            .push(button(&mut self.state_view_button, "State").on_press(Message::StateView))
            .push(button(&mut self.get_model_button, "Get sp model").on_press(Message::UpdateModel))
            .push(button(&mut self.make_goal_button, "Make goal").on_press(Message::DemoGoalView))
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Empty, // hmmm
    OperationView,
    IntentionView,
    StateView,
    DemoGoalView,
    ModelUpdate(Result<SPModelInfo, Error>),
    NewState(SPState),
    StateValueEdit(SPPath, String),
    UpdateModel,
    ResetOperation(SPPath),
    SetEstimatedCylinders,
    SetNotification(String, NotificationType),
    ClearNotification,
}

async fn set_state(
    client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    json: String,
) -> Result<(), Error> {
    let req_msg = r2r::sp_msgs::srv::Json::Request { json: json.clone() };
    let req = client.lock().unwrap().request(&req_msg)?;
    let resp = tokio::time::timeout(std::time::Duration::from_millis(500), req).await;
    if resp.is_err() {
        println!("state change request timed out: {}", json);
    }
    Ok(())
}

async fn get_model(
    client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
) -> Result<SPModelInfo, Error> {
    let req_msg = r2r::sp_msgs::srv::Json::Request::default();

    // jsut for testing
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let req = client.lock().unwrap().request(&req_msg)?;
    let resp = tokio::time::timeout(std::time::Duration::from_secs(1), req).await;
    if let Ok(Ok(msg)) = resp {
        #[derive(Debug, PartialEq, Clone, Default, Deserialize)]
        pub struct RunnerModel {
            pub compiled_model: CompiledModel,
            pub changes: Option<sp_domain::Model>,
        }

        let rm: RunnerModel = serde_json::from_str(&msg.json)?;
        Ok(SPModelInfo::from(rm.compiled_model))
    } else {
        return Err(Error::RosError);
    }
}

impl Application for SPOpViewer {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (SPOpViewer, Command<Message>) {
        let ctx = r2r::Context::create().expect("could not create context");
        let mut node = r2r::Node::create(ctx, "sp_op_viewer", "").expect("...");
        let get_model_client = Arc::new(Mutex::new(
            node.create_client::<r2r::sp_msgs::srv::Json::Service>("/sp/get_model")
                .expect("could not create client"),
        ));
        let set_state_client = Arc::new(Mutex::new(
            node.create_client::<r2r::sp_msgs::srv::Json::Service>("/sp/set_state")
                .expect("could not create client"),
        ));
        let sub = node
            .subscribe::<r2r::std_msgs::msg::String>("/sp/state", r2r::QosProfile::default())
            .expect("could not subscribe");

        let _handle = std::thread::spawn(move || loop {
            node.spin_once(std::time::Duration::from_millis(100));
        });

        let (sender, receiver) = tokio::sync::mpsc::channel::<SPState>(1);
        let _sub = tokio::spawn(async move {
            sub.for_each(|msg| {
                let s: SPStateJson = serde_json::from_str(&msg.data).expect("could not parse");
                let _res = sender.try_send(s.to_state());
                future::ready(())
            })
            .await;
        });

        (
            SPOpViewer {
                new_state_receiver: Mutex::new(Some(receiver)),
                get_model_client: get_model_client.clone(),
                set_state_client: set_state_client.clone(),
                ui_state: SPOpViewerState::Loading,
                notification: None,
            },
            Command::perform(get_model(get_model_client), Message::ModelUpdate),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Some(r) = self.new_state_receiver.lock().unwrap().take() {
            subscription::unfold(1, r, |mut r| async move {
                if let Some(new_state) = r.recv().await {
                    (Some(Message::NewState(new_state)), r)
                } else {
                    (None, r)
                }
            })
        } else {
            subscription::unfold(1, (), |_| async move { (None, ()) })
        }
    }

    fn title(&self) -> String {
        let subtitle = match &self.ui_state {
            SPOpViewerState::Loading => "Loading",
            SPOpViewerState::Loaded { .. } => "Model loaded",
            SPOpViewerState::Errored { .. } => "Error",
        };

        format!("SP Operation Viewer - {}", subtitle)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Empty => Command::none(),
            Message::SetNotification(msg, t) => {
                self.notification = Some(Notification::new(msg, t));
                Command::none()
            }
            Message::ClearNotification => {
                self.notification = None;
                Command::none()
            },
            Message::StateValueEdit(path, value) => {
                if let SPOpViewerState::Loaded {
                    model_info,
                    current_view: _,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    match model_info.state.iter_mut().find(|si| si.path == path) {
                        // TODO use the model to look up the correct type.
                        Some(ref mut si) => si.new_value = value,
                        None => (),
                    }
                }
                Command::none()
            }
            Message::StateView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    current_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *current_view = View::StateView;
                }
                Command::none()
            }
            Message::OperationView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    current_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *current_view = View::OperationView;
                }
                Command::none()
            }
            Message::IntentionView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    current_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *current_view = View::IntentionView;
                }
                Command::none()
            }
            Message::DemoGoalView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    current_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *current_view = View::DemoGoalView;
                }
                Command::none()
            }
            Message::ModelUpdate(Ok(model_info)) => {
                self.notification = Some(Notification::new("Model loaded!".to_string(),
                                                           NotificationType::Happy));
                self.ui_state = SPOpViewerState::Loaded {
                    model_info,
                    current_view: View::StateView,
                    scroll: scrollable::State::new(),
                    footer: Footer::default(),
                };

                Command::none()
            }
            Message::ModelUpdate(Err(_error)) => {
                self.ui_state = SPOpViewerState::Errored {
                    get_model_button: button::State::new(),
                };

                Command::none()
            }
            Message::NewState(s) => {
                if let SPOpViewerState::Loaded {
                    model_info,
                    current_view: _,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    // self.state = s;
                    let mut new_state = vec![];
                    for (k,v) in s.projection().state {
                        match model_info.state.iter_mut().find(|si| &si.path == k) {
                            Some(ref mut si) => si.value = v.value().clone(),
                            None => new_state.push(StateInfo {
                                path: k.clone(),
                                value: v.value().clone(),
                                new_value: String::new(),
                                new_value_state: text_input::State::new(),
                            }),
                        }
                    }
                    model_info.state.extend(new_state);
                }
                Command::none()
            }
            Message::UpdateModel => match self.ui_state {
                SPOpViewerState::Loading => Command::none(),
                _ => {
                    self.ui_state = SPOpViewerState::Loading;
                    Command::perform(
                        get_model(self.get_model_client.clone()),
                        Message::ModelUpdate,
                    )
                }
            },
            Message::ResetOperation(path) => {
                let new_state = SPState::new_from_values(&[(path, "i".to_spvalue())]);
                let new_state = SPStateJson::from_state_flat(&new_state);
                let json = new_state.to_json().to_string();
                Command::perform(set_state(self.set_state_client.clone(), json), |_| {
                    Message::SetNotification("Operation reset!".into(), NotificationType::Sad)
                })
            },
            Message::SetEstimatedCylinders => {
                if let SPOpViewerState::Loaded {
                    model_info,
                    current_view: _,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    let new_state = SPState::new_from_values(&[
                        (SPPath::from_string("/testpath"), "testvalue".to_spvalue())
                    ]);
                    let new_state = SPStateJson::from_state_flat(&new_state);
                    let json = new_state.to_json().to_string();
                    Command::perform(set_state(self.set_state_client.clone(), json), |_| {
                        Message::SetNotification("Updated the state".into(),
                                                 NotificationType::Neutral)
                    })
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let content = match &mut self.ui_state {
            SPOpViewerState::Loading => Column::new()
                .width(Length::Shrink)
                .push(Text::new("Waiting for model...").size(40)),
            SPOpViewerState::Loaded {
                model_info,
                current_view,
                scroll,
                footer,
            } => {
                let height = if self.notification.is_some() {
                    500
                } else {
                    600
                };
                let contents = Column::new()
                    .spacing(20)
                    .padding(10)
                    .push(Row::new()
                          .height(Length::Units(height))
                          .push(match current_view {
                              View::StateView => model_info.view_state(scroll),
                              View::OperationView => model_info.view_ops(),
                              View::IntentionView => model_info.view_ints(),
                              View::DemoGoalView => model_info.view_demo_goal(),
                          }))
                    .push(footer.view());
                if let Some(n) = self.notification.as_mut() {
                    contents.push(n.view())
                } else {
                    contents
                }
            },
            SPOpViewerState::Errored {
                get_model_button, ..
            } => Column::new()
                .spacing(20)
                .align_items(Alignment::End)
                .push(Text::new("Could not get model...").size(40))
                .push(button(get_model_button, "Try again").on_press(Message::UpdateModel)),
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    RosError,
    SerdeError,
}

impl From<r2r::Error> for Error {
    fn from(error: r2r::Error) -> Error {
        dbg!(error);

        Error::RosError
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        dbg!(error);

        Error::SerdeError
    }
}

fn button<'a>(state: &'a mut button::State, text: &str) -> Button<'a, Message> {
    Button::new(state, Text::new(text))
        .padding(10)
        .style(style::Button::Primary)
}

pub(crate) mod style {
    use iced::{button, Background, Color, Vector};

    pub enum Button {
        Primary,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::Color(match self {
                    Button::Primary => Color::from_rgb(0.11, 0.42, 0.87),
                })),
                border_radius: 12.0,
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: Color::WHITE,
                ..button::Style::default()
            }
        }
    }
}
