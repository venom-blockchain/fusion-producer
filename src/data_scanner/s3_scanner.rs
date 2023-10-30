use std::sync::Arc;

use anyhow::{Context, Result};
use archive_downloader::*;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};

use crate::archive::*;
use crate::blocks_handler::*;
use crate::config::*;

pub struct S3Scanner {
    handler: Arc<BlocksHandler>,
    downloader: ArchiveDownloader,
    retry_on_error: bool,
}

impl S3Scanner {
    pub async fn new(config: S3ScannerConfig, handler: Arc<BlocksHandler>) -> Result<Self> {
        let downloader = ArchiveDownloader::new(config.s3_config)
            .await
            .context("Failed to create S3 archive downloader")?;

        Ok(Self {
            handler,
            downloader,
            retry_on_error: config.retry_on_error,
        })
    }

    pub async fn run(self) -> Result<()> {
        let pb = ProgressBar::new_spinner();

        let total_style = ProgressStyle::default_bar()
            .template("Archives processed: {pos}. Speed: {per_sec}. {msg}")?;
        pb.set_style(total_style);

        let mut stream = self.downloader.archives_stream();
        while let Some(item) = stream.next().await {
            let (archive_name, archive): (String, Vec<u8>) =
                item.context("Failed to fetch archive")?;

            let parsed = parse_archive(archive).context("Invalid archive")?;
            for (block_id, parsed) in parsed {
                let (stuff, _data) = parsed.block_stuff;

                loop {
                    match self
                        .handler
                        .handle_block(
                            &stuff,
                            None
                        )
                        .await
                        .context("Failed to handle block")
                    {
                        Ok(()) => break,
                        Err(e) => {
                            pb.println(format!("Failed processing block {block_id}: {e:?}"));
                            if !self.retry_on_error {
                                return Err(e);
                            }
                        }
                    }
                }
            }

            pb.inc(1);
            pb.println(archive_name);
        }

        pb.println("Done");
        Ok(())
    }
}
