#[macro_use]
mod assert;
mod api;
mod mock;
mod tests;

use crate::api::{TestStep, TestStepContext};
use crate::tests::create_test_steps;
use reaper_rs_high::Reaper;
use rxrust::prelude::*;
use std::borrow::Cow;
use std::borrow::Cow::{Borrowed, Owned};

use std::collections::VecDeque;

use reaper_rs_medium::ReaperStringArg;
use std::ffi::CString;
use std::iter::FromIterator;
use std::time::Duration;

pub fn execute_integration_test() {
    let reaper = Reaper::get();
    reaper.clear_console();
    log("# Testing reaper-rs\n");
    let steps = VecDeque::from_iter(create_test_steps());
    let step_count = steps.len();
    execute_next_step(reaper, steps, step_count);
}

fn execute_next_step(reaper: &'static Reaper, mut steps: VecDeque<TestStep>, step_count: usize) {
    let step = match steps.pop_front() {
        Some(step) => step,
        None => {
            log("\n\n**Integration test was successful**");
            return;
        }
    };
    log_step(step_count - steps.len() - 1, step.name);
    let result = {
        let mut finished = LocalSubject::new();
        let context = TestStepContext {
            finished: finished.clone(),
        };
        let result = (step.operation)(reaper, context);
        finished.complete();
        result
    };
    match result {
        Ok(()) => {
            reaper.execute_later_in_main_thread_asap(move || {
                execute_next_step(reaper, steps, step_count)
            });
        }
        Err(msg) => log_failure(&msg),
    }
}

fn log_failure(msg: &str) {
    log(format!(" → **FAILED**\n\n{}", msg));
}

fn log_step(step_index: usize, name: impl Into<Cow<'static, str>>) {
    log(format!("\n{}. {}", step_index + 1, name.into()));
}

fn log<'a>(msg: impl Into<ReaperStringArg<'a>>) {
    Reaper::get().show_console_msg(msg)
}