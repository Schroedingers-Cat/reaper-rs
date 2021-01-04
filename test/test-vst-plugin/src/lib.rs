use futures_timer::Delay;
use reaper_high::{create_terminal_logger, ActionKind, CrashInfo, Reaper, ReaperGuard};
use reaper_low::{reaper_vst_plugin, static_vst_plugin_context, PluginContext};
use reaper_medium::{CommandId, ControlSurface, HookPostCommand, OnAudioBuffer, OnAudioBufferArgs};
use reaper_rx::{ControlSurfaceRx, ControlSurfaceRxMiddleware};
use rxrust::prelude::*;
use slog::debug;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;
use vst::plugin::{HostCallback, Info, Plugin};
use vst::plugin_main;

plugin_main!(TestVstPlugin);
reaper_vst_plugin!();

#[derive(Default)]
struct TestVstPlugin {
    host: HostCallback,
    session: Option<reaper_medium::ReaperSession>,
    reaper_guard: Option<Arc<ReaperGuard>>,
}

impl Plugin for TestVstPlugin {
    fn new(host: HostCallback) -> Self {
        Self {
            host,
            session: None,
            reaper_guard: None,
        }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "reaper-rs test".to_string(),
            unique_id: 8372,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        // self.use_medium_level_reaper();
        self.use_high_level_reaper();
    }
}

struct MyOnAudioBuffer {
    sender: std::sync::mpsc::Sender<String>,
    counter: u64,
}

impl OnAudioBuffer for MyOnAudioBuffer {
    fn call(&mut self, args: OnAudioBufferArgs) {
        if self.counter % 100 == 0 {
            self.sender
                .send(format!(
                    "Counter: {}, Args: {:?}, Channels: {:?}\n",
                    self.counter,
                    args,
                    (args.reg.input_nch(), args.reg.output_nch())
                ))
                .expect("couldn't send console logging message to main thread");
        }
        self.counter += 1;
    }
}

struct MyHookPostCommand;

impl HookPostCommand for MyHookPostCommand {
    fn call(command_id: CommandId, _flag: i32) {
        println!("Command {:?} executed", command_id)
    }
}

impl TestVstPlugin {
    // Exists for demonstration purposes and quick tests
    #[allow(dead_code)]
    fn use_medium_level_reaper(&mut self) {
        let context =
            PluginContext::from_vst_plugin(&self.host, static_vst_plugin_context()).unwrap();
        let low = reaper_low::Reaper::load(context);
        let mut med = reaper_medium::ReaperSession::new(low);
        {
            let (sender, receiver) = channel::<String>();
            med.reaper()
                .show_console_msg("Registering control surface ...");
            #[derive(Debug)]
            struct MyControlSurface {
                reaper: reaper_medium::Reaper,
                receiver: Receiver<String>,
            }

            impl ControlSurface for MyControlSurface {
                fn run(&mut self) {
                    for msg in self.receiver.try_iter() {
                        self.reaper.show_console_msg(msg);
                    }
                }

                fn set_track_list_change(&self) {
                    println!("Track list changed!")
                }
            }
            med.plugin_register_add_csurf_inst(Box::new(MyControlSurface {
                reaper: med.reaper().clone(),
                receiver,
            }))
            .expect("couldn't register control surface");
            med.reaper().show_console_msg("Registering action ...");
            med.plugin_register_add_hook_post_command::<MyHookPostCommand>()
                .expect("couldn't register hook post command");
            med.audio_reg_hardware_hook_add(Box::new(MyOnAudioBuffer { sender, counter: 0 }))
                .expect("couldn't register audio hook");
        }
        self.session = Some(med);
    }

    fn use_high_level_reaper(&mut self) {
        let guard = Reaper::guarded(
            || {
                let context =
                    PluginContext::from_vst_plugin(&self.host, static_vst_plugin_context())
                        .unwrap();
                Reaper::setup_with_defaults(
                    context,
                    create_terminal_logger(),
                    CrashInfo {
                        plugin_name: "reaper-rs test VST plug-in".to_string(),
                        plugin_version: env!("CARGO_PKG_VERSION").to_string(),
                        support_email_address: "info@helgoboss.org".to_string(),
                    },
                );
                let reaper = Reaper::get();
                reaper.wake_up().unwrap();
                debug!(
                    reaper.logger(),
                    "Loaded reaper-rs integration test VST plugin"
                );
                reaper.register_action(
                    "reaperRsVstIntegrationTests",
                    "reaper-rs VST integration tests",
                    || reaper_test::execute_integration_test(|_| ()),
                    ActionKind::NotToggleable,
                );
            },
            || || {},
        );
        self.reaper_guard = Some(guard);
        // Some Rx stuff
        #[derive(Debug)]
        struct CustomControlSurface {
            rx_driver: ControlSurfaceRxMiddleware,
        }
        impl ControlSurface for CustomControlSurface {
            fn run(&mut self) {
                self.rx_driver.run();
            }
        }
        impl CustomControlSurface {
            fn new(rx: ControlSurfaceRx) -> Self {
                Self {
                    rx_driver: ControlSurfaceRxMiddleware::new(rx),
                }
            }
        }
        let mut counter = 0;
        let control_surface_rx = ControlSurfaceRx::new();
        let control_surface = CustomControlSurface::new(control_surface_rx.clone());
        let reaper = Reaper::get();
        // TODO-medium This should be unregistered when VST plug-in removed.
        reaper
            .medium_session()
            .plugin_register_add_csurf_inst(Box::new(control_surface))
            .unwrap();
        control_surface_rx.main_thread_idle().subscribe(move |_| {
            if counter > 10 {
                return;
            }
            Reaper::get().show_console_msg(format!("Main thread counter: {}\n", counter));
            counter += 1;
        });
        // Some future stuff
        reaper.spawn_in_main_thread(future_main());
    }
}

async fn future_main() {
    Reaper::get().show_console_msg("Hello from future!\n");
    let result = calculate_something().await;
    Reaper::get().show_console_msg(format!("Calculated: {}\n", result));
    let result = calculate_something_else().await;
    Reaper::get().show_console_msg(format!("Calculated something else: {}\n", result));
}

async fn calculate_something() -> i32 {
    Reaper::get().show_console_msg("Calculating something...\n");
    Delay::new(Duration::from_secs(3)).await;
    5
}

async fn calculate_something_else() -> i32 {
    Reaper::get().show_console_msg("Calculating something else...\n");
    Delay::new(Duration::from_secs(5)).await;
    10
}
