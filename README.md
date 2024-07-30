<h1 align="center">
  <a href="https://paradedb.com"><img src="docs/logo/readme.svg" alt="ParadeDB" width="368px"></a>
<br>
</h1>

<p align="center">
  <b>Postgres for Search and Analytics</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Docs</a> &bull;
  <a href="https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ">Community</a> &bull;
  <a href="https://blog.paradedb.com">Blog</a> &bull;
  <a href="https://docs.paradedb.com/changelog/">Changelog</a>
</h3>

---

[![Publish ParadeDB](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml)
[![Artifact Hub](https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/paradedb)](https://artifacthub.io/packages/search?repo=paradedb)
[![Docker Pulls](https://img.shields.io/docker/pulls/paradedb/paradedb)](https://hub.docker.com/r/paradedb/paradedb)
[![License](https://img.shields.io/github/license/paradedb/paradedb?color=blue)](https://github.com/paradedb/paradedb?tab=AGPL-3.0-1-ov-file#readme)
[![Slack URL](https://img.shields.io/badge/Join%20Slack-purple?logo=slack&link=https%3A%2F%2Fjoin.slack.com%2Ft%2Fparadedbcommunity%2Fshared_invite%2Fzt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ)](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ)
[![X URL](https://img.shields.io/twitter/url?url=https%3A%2F%2Ftwitter.com%2Fparadedb&label=Follow%20%40paradedb)](https://x.com/paradedb)

[ParadeDB](https://paradedb.com) is an Elasticsearch alternative built on Postgres. We're modernizing the features of Elasticsearch's product suite, starting with real-time search and analytics.

## Status

ParadeDB is currently in Public Beta. Star and watch this repository to get notified of updates.

### Roadmap

- [x] Search
  - [x] Full-text search with BM25 with [pg_search](https://github.com/paradedb/paradedb/tree/dev/pg_search#overview)
  - [x] Dense and sparse vector search with [pgvector](https://github.com/pgvector/pgvector#pgvector) & [pgvectorscale](https://github.com/timescale/pgvectorscale#pgvectorscale)
  - [x] Hybrid search
- [ ] Analytics
  - [x] An analytical query engine over any object store or table format with [pg_lakehouse](https://github.com/paradedb/paradedb/tree/dev/pg_lakehouse#overview)
  - [ ] Column-oriented table access method for fast analytics inside Postgres
  - [ ] High-volume data/Kafka ingest
- [x] Self-Hosted ParadeDB
  - [x] Docker image based on [bitnami/postgresql](https://hub.docker.com/r/bitnami/postgresql) & [deployment instructions](https://docs.paradedb.com/deploy/aws)
  - [x] Kubernetes Helm chart & [deployment instructions](https://docs.paradedb.com/deploy/helm)
- [x] Specialized Workloads
  - [x] Support for geospatial data with [PostGIS](https://github.com/postgis/postgis)

## Get Started

To get started, please visit our [documentation](https://docs.paradedb.com).

## Deploying ParadeDB

ParadeDB and its extensions are available as commercial software for installation on self-hosted Postgres deployment and via Docker and Kubernetes as standalone images. For more information, including enterprise features and support, please [contact us by email](mailto:sales@paradedb.com).

### Extensions

You can find prebuilt binaries for all ParadeDB extensions on Debian 11, Debian 12, Ubuntu 22.04, and Red Hat Enterprise Linux 9 for Postgres 14, 15 and 16 in the [GitHub Releases](https://github.com/paradedb/paradedb/releases/latest). We officially support Postgres 12 and above, and you can compile the extensions for other versions of Postgres by following the instructions in the respective extension's README.

For official support on non-Debian-based systems, please [contact us by email](mailto:sales@paradedb.com).

### Docker Image

To quickly get a ParadeDB instance up and running, simply pull and run the latest Docker image:

```bash
docker run --name paradedb paradedb/paradedb
```

This will start a ParadeDB instance with default user `postgres` and password `postgres`. You can then connect to the database using `psql`:

```bash
docker exec -it paradedb psql -U postgres
```

To install ParadeDB locally or on-premise, we recommend using our `docker-compose.yml` file. Alternatively, you can pass the appropriate environment variables to the `docker run` command, replacing the <> with your desired values:

```bash
docker run \
  --name paradedb \
  -e POSTGRESQL_USERNAME=<user> \
  -e POSTGRESQL_PASSWORD=<password> \
  -e POSTGRESQL_DATABASE=<dbname> \
  -e POSTGRESQL_POSTGRES_PASSWORD=<superuser_password> \
  -v paradedb_data:/bitnami/postgresql \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

This will start a ParadeDB instance with non-root user `<user>` and password `<password>`. The `superuser_password` will be associated with superuser `postgres` and is necessary for ParadeDB extensions to install properly.

The `-v` flag enables your ParadeDB data to persist across restarts in a Docker volume named `paradedb_data`. The volume needs to be writable by a user with `uid = 1001`, which is a security requirement of the Bitnami PostgreSQL Docker image. You can do so with:

```bash
sudo useradd -u 1001 <user>
sudo chown <user> </path/to/paradedb_data>
```

You can then connect to the database using `psql`:

```bash
docker exec -it paradedb psql -U <user> -d <dbname> -p 5432 -W
```

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt out of telemetry using configuration variables within Postgres:

```sql
ALTER SYSTEM SET paradedb.pg_search_telemetry TO 'off';
ALTER SYSTEM SET paradedb.pg_lakehouse_telemetry TO 'off';
```

### Helm Chart

ParadeDB is also available for Kubernetes via our Helm chart. You can find our Helm chart in the [ParadeDB Helm Chart GitHub repository](https://github.com/paradedb/helm-charts) or download it directly from [Artifact Hub](https://artifacthub.io/packages/helm/paradedb/paradedb).

### ParadeDB Cloud

At the moment, ParadeDB is not available as a managed cloud service. If you are interested in a ParadeDB Cloud service, please let us know by joining our [waitlist](https://form.typeform.com/to/jHkLmIzx).

## Support

If you're missing a feature or have found a bug, please open a
[GitHub Issue](https://github.com/paradedb/paradedb/issues/new/choose).

To get community support, you can:

- Post a question in the [ParadeDB Slack Community](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ)
- Ask for help on our [GitHub Discussions](https://github.com/paradedb/paradedb/discussions)

If you need commercial support, please [contact the ParadeDB team](mailto:sales@paradedb.com).

## Contributing

We welcome community contributions, big or small, and are here to guide you along
the way. To get started contributing, check our [first timer issues](https://github.com/paradedb/paradedb/labels/good%20first%20issue)
or message us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ). Once you contribute, ping us in Slack and we'll send you some ParadeDB swag!

For more information on how to contribute, please see our
[Contributing Guide](/CONTRIBUTING.md).

This project is released with a [Contributor Code of Conduct](/CODE_OF_CONDUCT.md).
By participating in this project, you agree to follow its terms.

Thank you for helping us make ParadeDB better for everyone :heart:.

## License

ParadeDB is licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software. For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).
