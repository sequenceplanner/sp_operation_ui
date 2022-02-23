use futures::future;
use futures::stream::StreamExt;
use iced::{
    button, scrollable, Alignment, Application, Button, Column, Command, Container, Element, Length, Row,
    Settings, Subscription, Text, Scrollable
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
    SPOpViewer::run(Settings::default())
}

struct SPOpViewer {
    // ros communication stuff
    get_model_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    set_state_client: Arc<Mutex<r2r::Client<r2r::sp_msgs::srv::Json::Service>>>,
    new_state_receiver: Mutex<Option<tokio::sync::mpsc::Receiver<SPState>>>,

    // the sp state
    state: SPState,

    // our ui state
    ui_state: SPOpViewerState,
}

#[derive(Debug, Clone)]
enum SPOpViewerState {
    Loading,
    Loaded {
        model_info: SPModelInfo,
        intention_view: bool,
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
    get_model_button: button::State,
}

impl Footer {
    fn view(&mut self) -> Element<Message> {
        Row::new()
            .spacing(20)
            .width(Length::Fill)
            .align_items(Alignment::End)
            .push(button(&mut self.op_view_button, "Operations").on_press(Message::OperationView))
            .push(button(&mut self.int_view_button, "Intentions").on_press(Message::IntentionView))
            .push(button(&mut self.get_model_button, "Get sp model").on_press(Message::UpdateModel))
            .into()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Empty, // hmmm
    OperationView,
    IntentionView,
    ModelUpdate(Result<SPModelInfo, Error>),
    NewState(SPState),
    UpdateModel,
    ResetOperation(SPPath),
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
                state: SPState::new(),
                new_state_receiver: Mutex::new(Some(receiver)),
                get_model_client: get_model_client.clone(),
                set_state_client: set_state_client.clone(),
                ui_state: SPOpViewerState::Loading,
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
            Message::OperationView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    intention_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *intention_view = false;
                }
                Command::none()
            }
            Message::IntentionView => {
                if let SPOpViewerState::Loaded {
                    model_info: _,
                    intention_view,
                    scroll: _,
                    footer: _,
                } = &mut self.ui_state
                {
                    *intention_view = true;
                }
                Command::none()
            }
            Message::ModelUpdate(Ok(model_info)) => {
                self.ui_state = SPOpViewerState::Loaded {
                    model_info,
                    intention_view: false,
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
                self.state = s;
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
                    Message::Empty
                })
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
                intention_view,
                scroll,
                footer,
            } => Column::new()
                .max_width(500)
                .spacing(20)
                .align_items(Alignment::End)
                .push(model_info.view(&self.state, *intention_view))

                .push(
                    Scrollable::new(scroll)
                        .padding(40)
                        .max_height(400)
                        .push(Container::new(view_state(&self.state))))

                .push(footer.view()),
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
enum Error {
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

mod style {
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
