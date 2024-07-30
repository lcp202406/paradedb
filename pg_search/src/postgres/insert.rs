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

use crate::index::SearchIndex;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::utils::row_to_search_document;
use crate::writer::WriterDirectory;
use crate::{env::register_commit_callback, globals::WriterGlobal};
use pgrx::*;

#[allow(clippy::too_many_arguments)]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    _index_unchanged: bool,
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    let pg_relation = unsafe { PgRelation::from_pg(index_relation) };
    let rdopts: PgBox<SearchIndexCreateOptions> = if !pg_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(pg_relation.rd_options as *mut SearchIndexCreateOptions) }
    } else {
        let ops = unsafe { PgBox::<SearchIndexCreateOptions>::alloc0() };
        ops.into_pg_boxed()
    };

    let uuid = rdopts
        .get_uuid()
        .expect("uuid not specified in 'create_bm25' index build, please rebuild pg_search index");

    aminsert_internal(index_relation, values, isnull, heap_tid, &uuid)
}

#[cfg(any(feature = "pg12", feature = "pg13"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    let rdopts = (*index_relation).rd_options as *mut SearchIndexCreateOptions;

    let uuid = unsafe { rdopts.as_ref() }
        .expect("index rd_options are unexpectedly null")
        .get_uuid()
        .expect("uuid not specified in 'create_bm25' index build, please rebuild pg_search index");

    aminsert_internal(index_relation, values, isnull, heap_tid, &uuid)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: pg_sys::ItemPointer,
    uuid: &str,
) -> bool {
    let index_relation_ref: PgRelation = PgRelation::from_pg(index_relation);
    let tupdesc = index_relation_ref.tuple_desc();
    let index_name = index_relation_ref.name();
    let directory = WriterDirectory::from_index_name(index_name);
    let search_index = SearchIndex::from_cache(&directory, uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
    let search_document =
        row_to_search_document(*ctid, &tupdesc, values, isnull, &search_index.schema)
            .unwrap_or_else(|err| {
                panic!("error creating index entries for index '{index_name}': {err}",)
            });

    let writer_client = WriterGlobal::client();
    register_commit_callback(&writer_client, search_index.directory.clone())
        .expect("could not register commit callbacks for insert operation");

    search_index
        .insert(&writer_client, search_document)
        .unwrap_or_else(|err| panic!("error inserting document during insert callback: {err:?}"));

    true
}
