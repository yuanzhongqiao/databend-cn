// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use common_base::base::tokio::sync::mpsc::Receiver;
use common_catalog::table_context::TableContext;
use common_exception::Result;
use common_expression::Chunk;
use common_pipeline_core::processors::port::OutputPort;
use common_pipeline_core::processors::processor::ProcessorPtr;

use crate::processors::sources::SyncSource;
use crate::processors::sources::SyncSourcer;

#[allow(dead_code)]
pub struct SyncReceiverSource {
    receiver: Receiver<Result<Chunk<String>>>,
}

impl SyncReceiverSource {
    pub fn create(
        ctx: Arc<dyn TableContext>,
        rx: Receiver<Result<Chunk<String>>>,
        out: Arc<OutputPort>,
    ) -> Result<ProcessorPtr> {
        SyncSourcer::create(ctx, out, SyncReceiverSource { receiver: rx })
    }
}

#[async_trait::async_trait]
impl SyncSource for SyncReceiverSource {
    const NAME: &'static str = "SyncReceiverSource";

    fn generate(&mut self) -> Result<Option<Chunk<String>>> {
        match self.receiver.blocking_recv() {
            None => Ok(None),
            Some(Err(cause)) => Err(cause),
            Some(Ok(chunk)) => Ok(Some(chunk)),
        }
    }
}
