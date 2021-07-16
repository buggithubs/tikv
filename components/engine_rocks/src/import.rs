// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use crate::engine::RocksEngine;
use crate::util;
use engine_traits::ImportExt;
use engine_traits::IngestExternalFileOptions;
use engine_traits::Result;
use rocksdb::set_external_sst_file_global_seq_no;
use rocksdb::IngestExternalFileOptions as RawIngestExternalFileOptions;
use std::fs::File;

impl ImportExt for RocksEngine {
    type IngestExternalFileOptions = RocksIngestExternalFileOptions;

    fn ingest_external_file_cf(&self, cf: &str, files: &[&str]) -> Result<()> {
        let cf = util::get_cf_handle(self.as_inner(), cf)?;
        let mut opts = RocksIngestExternalFileOptions::new();
        opts.move_files(true);
        opts.set_write_global_seqno(false);
        files.iter().try_for_each(|file| -> Result<()> {
            let f = File::open(file)?;
            // Prior to v5.2.0, TiKV use `write_global_seqno=true` for ingestion. For backward
            // compatibility, in case TiKV is retrying an ingestion job generated by older
            // version, it needs to reset the global seqno to 0.
            set_external_sst_file_global_seq_no(&self.as_inner(), cf, file, 0)?;
            f.sync_all()
                .map_err(|e| format!("sync {}: {:?}", file, e))?;
            Ok(())
        })?;
        // This is calling a specially optimized version of
        // ingest_external_file_cf. In cases where the memtable needs to be
        // flushed it avoids blocking writers while doing the flush. The unused
        // return value here just indicates whether the fallback path requiring
        // the manual memtable flush was taken.
        let _did_nonblocking_memtable_flush = self
            .as_inner()
            .ingest_external_file_optimized(&cf, &opts.0, files)?;
        Ok(())
    }
}

pub struct RocksIngestExternalFileOptions(RawIngestExternalFileOptions);

impl IngestExternalFileOptions for RocksIngestExternalFileOptions {
    fn new() -> RocksIngestExternalFileOptions {
        RocksIngestExternalFileOptions(RawIngestExternalFileOptions::new())
    }

    fn move_files(&mut self, f: bool) {
        self.0.move_files(f);
    }

    fn get_write_global_seqno(&self) -> bool {
        self.0.get_write_global_seqno()
    }

    fn set_write_global_seqno(&mut self, f: bool) {
        self.0.set_write_global_seqno(f);
    }
}
