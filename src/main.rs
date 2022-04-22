use futures::future;
use futures::stream::StreamExt;
use iced::{
    window, button, scrollable, text_input, Alignment, Application, Button, Column, Command, Container, Element, Length, Row, Settings, Subscription, Text, TextInput
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
                size: (750, 720),
                ..window::Settings::default()
            },
            ..Settings::default()
        })
}

struct SPOpViewer {
    // ros communication stuff
    get_model_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    set_model_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    set_state_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    new_state_receiver: Mutex<Option<tokio::sync::mpsc::Receiver<SPState>>>,

    // our ui state
    ui_state: SPOpViewerState,

    // other state...
    notification: Option<Notification>,

    // global filter textbox
    filter_string: String,
    filter_edit_state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum View {
    OperationView,
    IntentionView,
    TPlanView,
    OPlanView,
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
    tplan_view_button: button::State,
    oplan_view_button: button::State,
    state_view_button: button::State,
    get_model_button: button::State,
    make_goal_button: button::State,
}

impl Footer {
    fn view(&mut self) -> Element<Message> {
        Row::new()
            .spacing(20)
            .push(button(&mut self.int_view_button, "Intentions")
                  .on_press(Message::ChangeView(View::IntentionView)))
            .push(button(&mut self.oplan_view_button, "O. Plan")
                  .on_press(Message::ChangeView(View::OPlanView)))
            .push(button(&mut self.op_view_button, "Operations")
                  .on_press(Message::ChangeView(View::OperationView)))
            .push(button(&mut self.tplan_view_button, "T. Plan")
                  .on_press(Message::ChangeView(View::TPlanView)))
            .push(button(&mut self.state_view_button, "State")
                  .on_press(Message::ChangeView(View::StateView)))
            .push(button(&mut self.make_goal_button, "Make goal")
                  .on_press(Message::ChangeView(View::DemoGoalView)))
            .push(button(&mut self.get_model_button, "Get sp model")
                  .on_press(Message::UpdateModel))
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum BufferLocationType {
    Estimated,
    Goal,
}

#[derive(Debug, Clone)]
pub enum Message {
    Empty, // hmmm
    ChangeView(View),
    ModelUpdate(Result<SPModelInfo, Error>),
    BufferButton(BufferLocationType, usize, bool),
    NewState(SPState),
    StateValueEdit(SPPath, String),
    UpdateModel,
    ResetOperation(SPPath, SPValue),
    SetEstimatedCylinders,
    SendGoalCylinders,
    SetNotification(String, NotificationType),
    ClearNotification,
    FilterChanged(String),
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

async fn set_model(
    client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    model: Model,
) -> Result<(), Error> {
    // let req_msg = r2r::sp_msgs::srv::Json::Request::default();
    let json = serde_json::to_string(&model).expect("could not serialize");
    let req_msg = r2r::sp_msgs::srv::Json::Request { json };

    let req = client.lock().unwrap().request(&req_msg)?;
    let resp = tokio::time::timeout(std::time::Duration::from_secs(1), req).await;
    if let Ok(Ok(msg)) = resp {
        println!("msg.json: {}", msg.json);
        Ok(())
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
        let set_model_client = Arc::new(Mutex::new(
            node.create_client::<r2r::sp_msgs::srv::Json::Service>("/sp/set_model")
                .expect("could not create client"),
        ));
        let set_state_client = Arc::new(Mutex::new(
            node.create_client::<r2r::sp_msgs::srv::Json::Service>("/sp/set_state")
                .expect("could not create client"),
        ));
        let sub = node
            .subscribe::<r2r::std_msgs::msg::String>("/sp/state_flat", r2r::QosProfile::default())
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
                set_model_client: set_model_client.clone(),
                set_state_client: set_state_client.clone(),
                ui_state: SPOpViewerState::Loading,
                notification: None,
                filter_string: String::new(),
                filter_edit_state: text_input::State::new(),
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
                Command::perform(
                    tokio::time::sleep(std::time::Duration::from_millis(2500)), |_| {
                    Message::ClearNotification
                })
            }
            Message::ClearNotification => {
                self.notification = None;
                Command::none()
            },
            Message::FilterChanged(s) => {
                self.filter_string = s;
                Command::none()
            },
            Message::BufferButton(type_, idx, val) => {
                if let SPOpViewerState::Loaded {
                    model_info,
                    current_view: _,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    let bl = match type_ {
                        BufferLocationType::Estimated => &mut model_info.estimated_locations,
                        BufferLocationType::Goal => &mut model_info.buffers_locations,
                    };

                    match bl.get_mut(idx) {
                        Some(ref mut bl) => bl.value = val,
                        None => (),
                    }
                }
                Command::none()
            }
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
            Message::ChangeView(view) => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    current_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *current_view = view;
                }
                Command::none()
            }
            Message::ModelUpdate(Ok(model_info)) => {
                self.ui_state = SPOpViewerState::Loaded {
                    model_info,
                    current_view: View::StateView,
                    scroll: scrollable::State::new(),
                    footer: Footer::default(),
                };

                // hack... dont know how to send again here
                self.notification = Some(Notification::new("Model loaded!".to_string(),
                                                           NotificationType::Happy));
                Command::perform(
                    tokio::time::sleep(std::time::Duration::from_millis(2500)), |_| {
                    Message::ClearNotification
                })

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
            Message::ResetOperation(path, change_to) => {
                let new_state = SPState::new_from_values(&[(path.clone(), change_to.clone())]);
                let new_state = SPStateJson::from_state_flat(&new_state);
                let json = new_state.to_json().to_string();
                Command::perform(set_state(self.set_state_client.clone(), json), move |_| {
                    Message::SetNotification(format!("{} set to {}!", path, change_to),
                                             NotificationType::Happy)
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
                    let updated_state: Vec<_> = model_info.estimated_locations
                        .iter()
                        .enumerate()
                        .map(|(i, bi)| {
                            let prefix = "lab_scenario_1/product_state";
                            let path = SPPath::from_string(&format!("{}/buffer{}",prefix,i+1));
                            (path, bi.value.to_spvalue())
                        }).collect();

                    let new_state = SPState::new_from_values(&updated_state);
                    let new_state = SPStateJson::from_state_flat(&new_state);
                    let json = new_state.to_json().to_string();
                    Command::perform(set_state(self.set_state_client.clone(), json), |_| {
                        Message::SetNotification("Updated the state".into(),
                                                 NotificationType::Neutral)
                    })
                } else {
                    Command::none()
                }
            },
            Message::SendGoalCylinders => {
                if let SPOpViewerState::Loaded {
                    model_info,
                    current_view: _,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    let predicate: Vec<_> = model_info.buffers_locations
                        .iter()
                        .enumerate()
                        .map(|(i, bi)| {
                            let prefix = "lab_scenario_1/product_state";
                            let path = SPPath::from_string(&format!("{}/buffer{}",prefix,i+1));

                            Predicate::EQ(PredicateValue::SPPath(path, None),
                                          PredicateValue::SPValue(bi.value.to_spvalue()))
                        }).collect();
                    let post = Predicate::AND(predicate);
                    let mut model = Model::new("lab_scenario_1");
                        model.add_intention("test_intention", false,
                                            &Predicate::TRUE,
                                            &post,
                                            &[]);
                    Command::perform(set_model(self.set_model_client.clone(), model), move |_| {
                        Message::SetNotification(//"Updated the state".into(),
                            format!("new intention: {}", post),
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
                          .push(
                              TextInput::new(
                                  &mut self.filter_edit_state,
                                  "Filter...",
                                  &self.filter_string,
                                  |s| Message::FilterChanged(s))))
                    .push(Row::new()
                          .height(Length::Units(height))
                          .push(match current_view {
                              View::StateView => model_info.view_state(&self.filter_string, scroll),
                              View::OperationView => model_info.view_ops(),
                              View::IntentionView => model_info.view_ints(),
                              View::TPlanView => model_info.view_tplan(),
                              View::OPlanView => model_info.view_oplan(),
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
        .padding(8)
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
                border_radius: 4.0,
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: Color::WHITE,
                ..button::Style::default()
            }
        }
    }
}
