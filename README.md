<p align="center">
  <img height="100" src="https://github.com/qdrant/qdrant/raw/master/docs/logo.svg" alt="Qdrant">
</p>

<p align="center">
    <b>Vector Search Engine for the next generation of AI applications</b>
</p>


<p align=center>
    <a href="https://github.com/qdrant/qdrant/actions/workflows/rust.yml"><img src="https://github.com/qdrant/qdrant/workflows/Tests/badge.svg" alt="Tests status"></a>
    <a href="https://qdrant.github.io/qdrant/redoc/index.html"><img src="https://img.shields.io/badge/Docs-OpenAPI%203.0-success" alt="OpenAPI Docs"></a>
    <a href="https://github.com/qdrant/qdrant/blob/master/LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-success" alt="Apache 2.0 License"></a>
    <a href="https://qdrant.to/discord"><img src="https://img.shields.io/badge/Discord-Qdrant-5865F2.svg?logo=discord" alt="Discord"></a>
    <a href="https://qdrant.to/roadmap"><img src="https://img.shields.io/badge/Roadmap-2023-bc1439.svg" alt="Roadmap 2023"></a>
</p>

**Qdrant** (read: _quadrant_ ) is a vector similarity search engine and vector database.
It provides a production-ready service with a convenient API to store, search, and manage points - vectors with an additional payload.
Qdrant is tailored to extended filtering support.  It makes it useful for all sorts of neural-network or semantic-based matching, faceted search, and other applications. 

Qdrant is written in Rust 🦀, which makes it fast and reliable even under high load.

With Qdrant, embeddings or neural network encoders can be turned into full-fledged applications for matching, searching, recommending, and much more!

Also available as managed solution in the **Qdrant Cloud** https://cloud.qdrant.io/ ⛅

## Demo Projects

### Semantic Text Search 🔍

The neural search uses semantic embeddings instead of keywords and works best with short texts.
With Qdrant and a pre-trained neural network, you can build and deploy semantic neural search on your data in minutes.
[Try it online!](https://qdrant.to/semantic-search-demo)

### Similar Image Search - Food Discovery 🍕

There are multiple ways to discover things, text search is not the only one.
In the case of food, people rely more on appearance than description and ingredients.
So why not let people choose their next lunch by its appearance, even if they don’t know the name of the dish?
[Check it out!](https://qdrant.to/food-discovery)

### Extreme classification - E-commerce Product Categorization 📺

Extreme classification is a rapidly growing research area within machine learning focusing on multi-class and multi-label problems involving an extremely large number of labels.
Sometimes it is millions and tens of millions classes.
The most promising way to solve this problem is to use similarity learning models.
We put together a demo example of how you could approach the problem with a pre-trained transformer model and Qdrant.
So you can [play with it online!](https://qdrant.to/extreme-classification-demo)


<details>
<summary> More solutions </summary>

<table>
    <tr>
        <td width="30%">
            <img src="https://qdrant.tech/content/images/text_search.png">
        </td>
        <td width="30%">
            <img src="https://qdrant.tech/content/images/image_search.png">
        </td>
        <td width="30%">
            <img src="https://qdrant.tech/content/images/recommendations.png">
        </td>
    </tr>
    <tr>
        <td>
            Semantic Text Search
        </td>
        <td>
            Similar Image Search
        </td>
        <td>
            Recommendations
        </td>
    </tr>
</table>

<table>
    <tr>
        <td>
            <img width="300px" src="https://qdrant.tech/content/images/chat_bots.png">
        </td>
        <td>
            <img width="300px" src="https://qdrant.tech/content/images/matching_engines.png">
        </td>
        <td>
            <img width="300px" src="https://qdrant.tech/content/images/anomalies_detection.png">
        </td>
    </tr>
    <tr>
        <td>
            Chat Bots
        </td>
        <td>
            Matching Engines
        </td>
        <td>
            Anomaly Detection
        </td>
    </tr>
</table>

</details>

## API
### REST

Online OpenAPI 3.0 documentation is available [here](https://qdrant.github.io/qdrant/redoc/index.html).
OpenAPI makes it easy to generate a client for virtually any framework or programing language.

You can also download raw OpenAPI [definitions](https://github.com/qdrant/qdrant/blob/master/docs/redoc/master/openapi.json).

### gRPC

For faster production-tier searches, Qdrant also provides a gRPC interface. You can find gRPC documentation [here](https://qdrant.tech/documentation/quick_start/#grpc).

### Clients

Qdrant offers the following client libraries to help you integrate it into your application stack with ease:

- [Python client](https://github.com/qdrant/qdrant_client)
- [Go client](https://github.com/qdrant/go-client)
- [Rust client](https://github.com/qdrant/rust-client)

## Features

### Filtering and Payload

Qdrant supports any JSON payload associated with vectors. It does not only store payload but also allows filter results based on payload values.
It allows any combinations of `should`, `must`, and `must_not` conditions, but unlike ElasticSearch post-filtering, Qdrant guarantees all relevant vectors are retrieved.

### Rich Data Types

Vector payload supports a large variety of data types and query conditions, including string matching, numerical ranges, geo-locations, and more.
Payload filtering conditions allow you to build almost any custom business logic that should work on top of similarity matching.

### Query Planning and Payload Indexes

Using the information about the stored payload values, the `query planner` decides on the best way to execute the query.
For example, if the search space limited by filters is small, it is more efficient to use a full brute force than an index.

### SIMD Hardware Acceleration

Qdrant can take advantage of modern CPU x86-x64 architectures. 
It allows you to search even faster on modern hardware.

### Write-Ahead Logging

Once the service confirmed an update - it won't lose data even in case of power shut down. 
All operations are stored in the update journal and the latest database state could be easily reconstructed at any moment.

### Distributed Deployment

Since [v0.8.0](https://github.com/qdrant/qdrant/releases/tag/v0.8.0) Qdrant supports distributed deployment.
In this mode, multiple Qdrant machines are joined into a cluster to provide horizontal scaling.
Coordination with the distributed consensus is provided by the [Raft](https://raft.github.io/) protocol.

### Stand-alone

Qdrant does not rely on any external database or orchestration controller, which makes it very easy to configure.

## Usage

### Docker 🐳

Build your own from source

```bash
docker build . --tag=qdrant/qdrant
```

Or use latest pre-built image from [DockerHub](https://hub.docker.com/r/qdrant/qdrant)

```bash
docker pull qdrant/qdrant
```

To run the container, use the command:

```bash
docker run -p 6333:6333 qdrant/qdrant
```

And once you need a fine-grained setup, you can also define a storage path and custom configuration:

```bash
docker run -p 6333:6333 \
    -v $(pwd)/path/to/data:/qdrant/storage \
    -v $(pwd)/path/to/custom_config.yaml:/qdrant/config/production.yaml \
    qdrant/qdrant
```

* `/qdrant/storage` - is a place where Qdrant persists all your data. 
Make sure to mount it as a volume, otherwise docker will drop it with the container. 
* `/qdrant/config/production.yaml` - is the file with engine configuration. You can override any value from the [reference config](https://github.com/qdrant/qdrant/blob/master/config/config.yaml) 

Now Qdrant should be accessible at [localhost:6333](http://localhost:6333/).

## Docs 📓

* The best place to start is [Quick Start Guide](https://github.com/qdrant/qdrant/blob/master/QUICK_START.md)
* The [Documentation](https://qdrant.tech/documentation/)
* Use the [OpenAPI specification](https://qdrant.github.io/qdrant/redoc/index.html) as a reference
* Follow our [Step-by-Step Tutorial](https://qdrant.to/qdrant-tutorial) to create your first neural network project with Qdrant
* Assess Qdrant's performance in our [benchmarks](https://qdrant.tech/benchmarks/)
* Check out our further plans in [v1.0 Roadmap](https://qdrant.to/roadmap)

## Contacts

* Join our [Discord channel](https://qdrant.to/discord)
* Follow us on [Twitter](https://qdrant.to/twitter)
* Subscribe to our [Newsletters](https://qdrant.to/newsletter)
* Write us an email [info@qdrant.tech](mailto:info@qdrant.tech)

Building something special with Qdrant? We can [help](https://qdrant.tech/pricing/)!

## Contributors ✨

Thanks to the people who contributed to Qdrant:

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://t.me/neural_network_engineering"><img src="https://avatars.githubusercontent.com/u/1935623?v=4?s=50" width="50px;" alt="Andrey Vasnetsov"/><br /><sub><b>Andrey Vasnetsov</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=generall" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/azayarni"><img src="https://avatars.githubusercontent.com/u/926368?v=4?s=50" width="50px;" alt="Andre Zayarni"/><br /><sub><b>Andre Zayarni</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=azayarni" title="Documentation">📖</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://www.linkedin.com/in/joanfontanalsmartinez/"><img src="https://avatars.githubusercontent.com/u/19825685?v=4?s=50" width="50px;" alt="Joan Fontanals"/><br /><sub><b>Joan Fontanals</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=JoanFM" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/trean"><img src="https://avatars.githubusercontent.com/u/7085263?v=4?s=50" width="50px;" alt="trean"/><br /><sub><b>trean</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=trean" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/kgrech"><img src="https://avatars.githubusercontent.com/u/9020133?v=4?s=50" width="50px;" alt="Konstantin"/><br /><sub><b>Konstantin</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=kgrech" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/kekonen"><img src="https://avatars.githubusercontent.com/u/11177808?v=4?s=50" width="50px;" alt="Daniil Naumetc"/><br /><sub><b>Daniil Naumetc</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=kekonen" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://dev.to/vearutop"><img src="https://avatars.githubusercontent.com/u/1381436?v=4?s=50" width="50px;" alt="Viacheslav Poturaev"/><br /><sub><b>Viacheslav Poturaev</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=vearutop" title="Documentation">📖</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/galibey"><img src="https://avatars.githubusercontent.com/u/48586936?v=4?s=50" width="50px;" alt="Alexander Galibey"/><br /><sub><b>Alexander Galibey</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=galibey" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/HaiCheViet"><img src="https://avatars.githubusercontent.com/u/37202591?v=4?s=50" width="50px;" alt="HaiCheViet"/><br /><sub><b>HaiCheViet</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=HaiCheViet" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://tranzystorek-io.github.io/"><img src="https://avatars.githubusercontent.com/u/5671049?v=4?s=50" width="50px;" alt="Marcin Puc"/><br /><sub><b>Marcin Puc</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=tranzystorek-io" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/anveq"><img src="https://avatars.githubusercontent.com/u/94402218?v=4?s=50" width="50px;" alt="Anton V."/><br /><sub><b>Anton V.</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=anveq" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://agourlay.github.io"><img src="https://avatars.githubusercontent.com/u/606963?v=4?s=50" width="50px;" alt="Arnaud Gourlay"/><br /><sub><b>Arnaud Gourlay</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=agourlay" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://t.me/type_driven_thoughts"><img src="https://avatars.githubusercontent.com/u/17401538?v=4?s=50" width="50px;" alt="Egor Ivkov"/><br /><sub><b>Egor Ivkov</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=eadventurous" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/IvanPleshkov"><img src="https://avatars.githubusercontent.com/u/20946825?v=4?s=50" width="50px;" alt="Ivan Pleshkov"/><br /><sub><b>Ivan Pleshkov</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=IvanPleshkov" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/daniilsunyaev"><img src="https://avatars.githubusercontent.com/u/3955599?v=4?s=50" width="50px;" alt="Daniil"/><br /><sub><b>Daniil</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=daniilsunyaev" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://homeonrails.com"><img src="https://avatars.githubusercontent.com/u/1282182?v=4?s=50" width="50px;" alt="Anton Kaliaev"/><br /><sub><b>Anton Kaliaev</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=melekes" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://soundcloud.com/norom"><img src="https://avatars.githubusercontent.com/u/7762532?v=4?s=50" width="50px;" alt="Andre Julius"/><br /><sub><b>Andre Julius</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=NotNorom" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/prok20"><img src="https://avatars.githubusercontent.com/u/20628026?v=4?s=50" width="50px;" alt="Prokudin Alexander"/><br /><sub><b>Prokudin Alexander</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=prok20" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/elbart"><img src="https://avatars.githubusercontent.com/u/48974?v=4?s=50" width="50px;" alt="Tim Eggert"/><br /><sub><b>Tim Eggert</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=elbart" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/gvelo"><img src="https://avatars.githubusercontent.com/u/943360?v=4?s=50" width="50px;" alt="Gabriel Velo"/><br /><sub><b>Gabriel Velo</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=gvelo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://burtonqin.github.io"><img src="https://avatars.githubusercontent.com/u/11943383?v=4?s=50" width="50px;" alt="Boqin Qin(秦 伯钦)"/><br /><sub><b>Boqin Qin(秦 伯钦)</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/issues?q=author%3ABurtonQin" title="Bug reports">🐛</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://forloop.co.uk/blog"><img src="https://avatars.githubusercontent.com/u/208231?v=4?s=50" width="50px;" alt="Russ Cam"/><br /><sub><b>Russ Cam</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=russcam" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/erare-humanum"><img src="https://avatars.githubusercontent.com/u/116254494?v=4?s=50" width="50px;" alt="erare-humanum"/><br /><sub><b>erare-humanum</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=erare-humanum" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/ffuugoo"><img src="https://avatars.githubusercontent.com/u/2725918?v=4?s=50" width="50px;" alt="Roman Titov"/><br /><sub><b>Roman Titov</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=ffuugoo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://hozan23.com"><img src="https://avatars.githubusercontent.com/u/119854621?v=4?s=50" width="50px;" alt="Hozan"/><br /><sub><b>Hozan</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=hozan23" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/joein"><img src="https://avatars.githubusercontent.com/u/22641570?v=4?s=50" width="50px;" alt="George"/><br /><sub><b>George</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=joein" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/csko"><img src="https://avatars.githubusercontent.com/u/749306?v=4?s=50" width="50px;" alt="Kornél Csernai"/><br /><sub><b>Kornél Csernai</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=csko" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://coszio.github.io"><img src="https://avatars.githubusercontent.com/u/62079184?v=4?s=50" width="50px;" alt="Luis Cossío"/><br /><sub><b>Luis Cossío</b></sub></a><br /><a href="https://github.com/qdrant/qdrant/commits?author=coszio" title="Documentation">📖</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

Qdrant is licensed under the Apache License, Version 2.0. View a copy of the [License file](https://github.com/qdrant/qdrant/blob/master/LICENSE).
