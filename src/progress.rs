/// Provides a progress bar that shows how many entries have been created. 

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use indicatif::ProgressBar;

// TODO: Transport messages via the channel. This allows for smoother display and 
// avoid weird left-overs when the bar moves down a line bcause we printed 
// a log statement.
pub type ProgressData = ();
pub type ProgressSender = UnboundedSender<ProgressData>;
pub type ProgressReceiver = UnboundedReceiver<ProgressData>;

pub async fn start_progress_task(max_count: u64) -> ProgressSender {
    let (tx, rx) = unbounded_channel();
    tokio::spawn(async move { progress_task(max_count, rx).await });

    tx
}

async fn progress_task(max_count: u64, rx: ProgressReceiver) {
    let bar = ProgressBar::new(max_count);
    let mut stream = UnboundedReceiverStream::new(rx);

    while let Some(_) = stream.next().await {
        bar.inc(1);
    }

    bar.finish();
}
