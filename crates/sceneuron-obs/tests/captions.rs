// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises stream captions against a stand-in output: lines split into CEA-608 pieces of two
//! rows within libobs's 128 bytes, scripts CEA-608 cannot show skipped, each piece paced by its
//! display time, a backlog trimmed to the newest pieces, the screen cleared after the clear delay,
//! nothing sent off the air, an idle sender that never touches the output and waits to be woken,
//! and each final line handed out once per language.

use std::cell::Cell;

use sceneuron::captions::{
    CaptionFeed, CaptionOutput, CaptionSender, MAX_CAPTION_BYTES, MAX_POLL_MS, MAX_WAITING, MIN_POLL_MS, ROW_CHARS,
    caption_pieces, display_seconds,
};

/// Records what it was sent and how often it was asked whether it is live; `streaming` and
/// `accepting` say whether it is live and takes text.
struct Output {
    streaming: bool,
    accepting: bool,
    sent: Vec<(String, f64)>,
    asked: Cell<usize>,
}

impl Output {
    fn live() -> Output {
        Output {
            streaming: true,
            accepting: true,
            sent: Vec::new(),
            asked: Cell::new(0),
        }
    }

    fn texts(&self) -> Vec<&str> {
        self.sent.iter().map(|(text, _)| text.as_str()).collect()
    }
}

impl CaptionOutput for Output {
    fn streaming(&self) -> bool {
        self.asked.set(self.asked.get() + 1);
        self.streaming
    }

    fn send(&mut self, text: &str, seconds: f64) -> bool {
        if self.accepting {
            self.sent.push((text.to_owned(), seconds));
        }
        self.accepting
    }
}

fn lines(pairs: &[(u64, &str)]) -> Vec<(u64, String)> {
    pairs.iter().map(|(id, text)| (*id, (*text).to_owned())).collect()
}

#[test]
fn lines_go_out_in_pieces_of_two_rows_broken_between_words() {
    let line = "Thanks for joining everyone, today we will look at the quarterly numbers and then the roadmap.";
    let pieces = caption_pieces(line);
    assert_eq!(
        pieces,
        [
            "Thanks for joining everyone, today we will look at the",
            "quarterly numbers and then the roadmap."
        ]
    );
    assert!(pieces.iter().all(|piece| piece.chars().count() <= 2 * ROW_CHARS + 1));
    assert_eq!(caption_pieces("¿Qué tal, señor?"), ["¿Qué tal, señor?"]);
    assert!(
        caption_pieces("みなさん、こんにちは").is_empty(),
        "CEA-608 carries Latin script only"
    );
    assert!(caption_pieces("   ").is_empty());
}

#[test]
fn a_piece_never_exceeds_what_libobs_keeps_and_is_cut_between_characters() {
    let line = "a€€€€€€ ".repeat(8);
    let pieces = caption_pieces(&line);
    assert!(pieces[0].len() <= MAX_CAPTION_BYTES, "{} bytes", pieces[0].len());
    assert!(line.starts_with(&pieces[0]), "a prefix of the line");
    assert!(
        pieces[0].len() > MAX_CAPTION_BYTES - 3,
        "cut at the last whole character"
    );
}

#[test]
fn each_piece_stays_by_its_length_between_one_and_a_half_and_four_seconds() {
    assert_eq!(display_seconds("Hi."), 1.5);
    assert_eq!(display_seconds(&"x".repeat(30)), 2.0);
    assert_eq!(display_seconds(&"x".repeat(64)), 4.0);
}

#[test]
fn pieces_go_out_one_at_a_time_once_the_one_before_had_its_time() {
    let mut sender = CaptionSender::new(4_000);
    let mut output = Output::live();
    sender.push("Good morning.");
    sender.push("Let's begin.");
    sender.poll(1_000, &mut output);
    assert_eq!(output.sent, [("Good morning.".to_owned(), 1.5)]);
    assert_eq!(sender.next_due_ms(), Some(2_500));
    sender.poll(2_499, &mut output);
    assert_eq!(output.sent.len(), 1, "the first piece has not had its time");
    sender.poll(2_500, &mut output);
    assert_eq!(output.texts(), ["Good morning.", "Let's begin."]);
}

#[test]
fn a_backlog_keeps_only_the_newest_pieces() {
    let mut sender = CaptionSender::new(4_000);
    let mut output = Output::live();
    for n in 1..=6 {
        sender.push(&format!("Line {n}."));
    }
    let mut now = 0;
    while let Some(due) = sender.next_due_ms().filter(|_| output.sent.len() < MAX_WAITING) {
        now = now.max(due);
        sender.poll(now, &mut output);
    }
    assert_eq!(output.texts(), ["Line 3.", "Line 4.", "Line 5.", "Line 6."]);
}

#[test]
fn the_screen_is_cleared_after_the_clear_delay_once() {
    let mut sender = CaptionSender::new(4_000);
    let mut output = Output::live();
    sender.push("Thanks, everyone.");
    sender.poll(10_000, &mut output);
    assert_eq!(sender.next_due_ms(), Some(14_000));
    sender.poll(13_999, &mut output);
    assert_eq!(output.sent.len(), 1);
    sender.poll(14_000, &mut output);
    assert_eq!(
        output.sent[1],
        (String::new(), 0.0),
        "an empty caption clears the screen"
    );
    sender.poll(20_000, &mut output);
    assert_eq!((output.sent.len(), sender.next_due_ms()), (2, None));
    sender.set_clear_after(1_000);
    sender.push(&"x".repeat(2 * 32));
    sender.poll(30_000, &mut output);
    assert_eq!(
        sender.next_due_ms(),
        Some(34_000),
        "never before the piece had its time"
    );
}

#[test]
fn nothing_goes_out_off_the_air_and_what_waited_is_dropped() {
    let mut sender = CaptionSender::new(4_000);
    let mut output = Output {
        streaming: false,
        ..Output::live()
    };
    sender.push("Before going live.");
    sender.poll(0, &mut output);
    assert_eq!((output.sent.len(), sender.next_due_ms()), (0, None));
    output.streaming = true;
    sender.poll(100, &mut output);
    assert!(
        output.sent.is_empty(),
        "lines from before the stream are not sent later"
    );
    output.accepting = false;
    sender.push("No output to take it.");
    sender.poll(200, &mut output);
    assert_eq!(sender.next_due_ms(), None, "a refused piece is not taken for shown");
}

#[test]
fn an_idle_sender_never_touches_the_output_and_waits_to_be_woken() {
    let mut sender = CaptionSender::new(4_000);
    let mut output = Output::live();
    sender.poll(0, &mut output);
    assert_eq!(
        (output.asked.get(), output.sent.len()),
        (0, 0),
        "nothing waits, so the output is never looked up"
    );
    assert_eq!(sender.wait_ms(0), None, "only a new line or a stop wakes the thread");
    sender.push("Hello.");
    assert_eq!(sender.wait_ms(0), Some(MIN_POLL_MS), "a waiting piece is due at once");
    sender.poll(1_000, &mut output);
    assert_eq!((output.asked.get(), output.texts()), (1, vec!["Hello."]));
    assert_eq!(
        sender.wait_ms(1_000),
        Some(MAX_POLL_MS),
        "the clear is due in 4 s, and a stream that stops is noticed within half a second"
    );
    sender.poll(5_000, &mut output);
    assert_eq!(output.texts(), ["Hello.", ""], "the screen is cleared");
    let asked = output.asked.get();
    sender.poll(9_000, &mut output);
    assert_eq!(output.asked.get(), asked, "idle again after the clear");
    assert_eq!(sender.wait_ms(9_000), None);
}

#[test]
fn each_final_line_is_handed_out_once_per_language_from_when_captions_start() {
    let mut feed = CaptionFeed::default();
    assert!(
        feed.new_lines("", &lines(&[(1, "Earlier.")])).is_empty(),
        "captions start with what comes next"
    );
    assert_eq!(
        feed.new_lines("", &lines(&[(1, "Earlier."), (2, "Hello.")])),
        ["Hello."]
    );
    assert!(feed.new_lines("", &lines(&[(1, "Earlier."), (2, "Hello.")])).is_empty());
    assert!(
        feed.new_lines("es", &lines(&[(2, "Hola.")])).is_empty(),
        "a new language starts afresh"
    );
    assert_eq!(feed.new_lines("es", &lines(&[(2, "Hola."), (4, "Adiós.")])), ["Adiós."]);
    assert_eq!(
        feed.new_lines("es", &lines(&[(2, "Hola."), (3, "Gracias."), (4, "Adiós.")])),
        ["Gracias."],
        "a translation that arrives late still goes out"
    );
    feed.reset();
    assert!(feed.new_lines("es", &lines(&[(5, "Otra.")])).is_empty());
}
