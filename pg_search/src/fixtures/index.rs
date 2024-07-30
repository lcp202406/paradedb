// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use uuid::Uuid;

use crate::{
    index::SearchIndex,
    schema::{SearchFieldConfig, SearchFieldName, SearchFieldType},
    writer::Writer,
};

use super::MockWriterDirectory;

pub struct MockSearchIndex {
    pub directory: MockWriterDirectory,
    pub index: &'static mut SearchIndex,
}

impl MockSearchIndex {
    pub fn new(
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Self {
        // We must store the TempDir instance on the struct, because it gets deleted when the
        // instance is dropped.
        let directory = MockWriterDirectory::new("mock_parade_search_index");
        let mut writer = Writer::new();
        let uuid = Uuid::new_v4().to_string();
        writer
            .create_index(directory.writer_dir.clone(), fields, uuid, key_field_index)
            .expect("error creating index instance");

        let index = SearchIndex::from_disk(&directory.writer_dir)
            .expect("error reading new index from cache");
        Self { directory, index }
    }
}
