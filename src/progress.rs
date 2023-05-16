/// Provides a progress bar that shows how many entries have been created. 

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio::time;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use indicatif::{ProgressBar, ProgressStyle, HumanDuration};

// TODO: Transport messages via the channel. This allows for smoother display and 
// avoid weird left-overs when the bar moves down a line bcause we printed 
// a log statement.
#[derive(Debug, Clone)]
pub enum ProgressMessage {
    Progress,
    ProgressWithMessage(String),
    Message(String)
}

pub type ProgressData = ProgressMessage;
pub type ProgressSender = UnboundedSender<ProgressData>;
pub type ProgressReceiver = UnboundedReceiver<ProgressData>;

pub async fn start_progress_task(max_count: u64) -> ProgressSender {
    let (tx, rx) = unbounded_channel();
    tokio::spawn(async move { progress_task(max_count, rx).await });

    tx
}

async fn progress_task(max_count: u64, rx: ProgressReceiver) {
    let style = ProgressStyle::with_template("{wide_bar} [{pos}/{len}] ({percent}%) {msg} [{eta}]").expect("valid style");
    let bar = ProgressBar::new(max_count);
    bar.set_style(style);
    let mut stream = UnboundedReceiverStream::new(rx);
    let mut count = 0;
    let start = time::Instant::now();
    let mut current_interval = start;
    let mut current_count = 0;

    while let Some(data) = stream.next().await {
        let inc = match data {
            ProgressMessage::Progress => 1,
            ProgressMessage::ProgressWithMessage(s) => {
                bar.println(s);
                1
            },
            ProgressMessage::Message(s) => {
                bar.println(s);
                0
            }
        };
        bar.inc(inc);
        count += inc;
        current_count += inc;

        let now = time::Instant::now();
        if (now - current_interval).as_secs() >= 1 {
            let msg = format!("{current_count} entries/second");
            bar.set_message(msg.clone());
            current_count = 0;
            current_interval = now;
        }
    }

    let end = time::Instant::now();
    let total_duration = end-start;
    let avg = count/total_duration.as_secs();
    let msg = format!("Created {avg} entries/second on average");
    bar.finish_with_message(msg);

}
