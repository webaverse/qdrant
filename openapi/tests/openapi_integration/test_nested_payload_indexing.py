import pytest
import json

from .helpers.helpers import request_with_validation
from .helpers.collection_setup import drop_collection

collection_name = 'test_collection_payload_indexing'


def nested_payload_collection_setup(collection_name, on_disk_payload=False):
    response = request_with_validation(
        api='/collections/{collection_name}',
        method="DELETE",
        path_params={'collection_name': collection_name},
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}',
        method="PUT",
        path_params={'collection_name': collection_name},
        body={
            "vectors": {
                "size": 4,
                "distance": "Dot"
            },
            "on_disk_payload": on_disk_payload
        }
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}',
        method="GET",
        path_params={'collection_name': collection_name},
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}/points',
        method="PUT",
        path_params={'collection_name': collection_name},
        query_params={'wait': 'true'},
        body={
            "points": [
                {
                    "id": 1,
                    "vector": [0.05, 0.61, 0.76, 0.74],
                    "payload": {
                        "country": {
                            "name": "Germany",
                            "capital": "Berlin",
                            "cities": [
                                {
                                    "name": "Berlin",
                                    "population": 3.7,
                                    "sightseeing": ["Brandenburg Gate", "Reichstag"]
                                },
                                {
                                    "name": "Munich",
                                    "population": 1.5,
                                    "sightseeing": ["Marienplatz", "Olympiapark"]
                                },
                                {
                                    "name": "Hamburg",
                                    "population": 1.8,
                                    "sightseeing": ["Reeperbahn", "Elbphilharmonie"]
                                }
                            ],
                        }
                    }
                },
                {
                    "id": 2,
                    "vector": [0.19, 0.81, 0.75, 0.11],
                    "payload": {
                        "country": {
                            "name": "England",
                            "capital": "London",
                            "cities": [
                                {
                                    "name": "London",
                                    "population": 8.9,
                                    "sightseeing": ["Big Ben", "London Eye"]
                                },
                                {
                                    "name": "Manchester",
                                    "population": 2.5,
                                    "sightseeing": ["Manchester United", "Manchester City"]
                                },
                                {
                                    "name": "Liverpool",
                                    "population": 0.5,
                                    "sightseeing": ["Anfield", "Albert Dock"]
                                }
                            ]
                        }
                    }
                },
                {
                    "id": 3,
                    "vector": [0.36, 0.55, 0.47, 0.94],
                    "payload": {
                        "country": {
                            "name": "France",
                            "capital": "Paris",
                            "cities": [
                                {
                                    "name": "Paris",
                                    "population": 2.2,
                                    "sightseeing": ["Eiffel Tower", "Louvre"]
                                },
                                {
                                    "name": "Marseille",
                                    "population": 0.9,
                                    "sightseeing": ["Vieux Port", "Notre Dame de la Garde"]
                                },
                                {
                                    "name": "Lyon",
                                    "population": 0.5,
                                    "sightseeing": ["Place Bellecour", "Fourvière Basilica"]
                                }
                            ]
                        }
                    }
                },
                {
                    "id": 4,
                    "vector": [0.18, 0.01, 0.85, 0.80],
                    "payload": {
                        "country": {
                            "name": "Japan",
                            "capital": "Tokyo",
                            "cities": [
                                {
                                    "name": "Tokyo",
                                    "population": 9.3,
                                    "sightseeing": ["Tokyo Tower", "Tokyo Skytree"]
                                },
                                {
                                    "name": "Osaka",
                                    "population": 2.7,
                                    "sightseeing": ["Osaka Castle", "Universal Studios Japan"]
                                },
                                {
                                    "name": "Kyoto",
                                    "population": 1.5,
                                    "sightseeing": ["Kiyomizu-dera", "Fushimi Inari-taisha"]
                                }
                            ]
                        }
                    }
                },
                {
                    "id": 5,
                    "vector": [0.24, 0.18, 0.22, 0.44],
                    "payload": {
                        "country": {
                            "name": "Nauru",
                        }
                    }
                },
                {
                    "id": 6,
                    "vector": [0.35, 0.08, 0.11, 0.44]
                }
            ]
        }
    )
    assert response.ok


@pytest.fixture(autouse=True)
def setup():
    nested_payload_collection_setup(collection_name=collection_name)
    yield
    drop_collection(collection_name=collection_name)


def test_payload_indexing_operations():
    response = request_with_validation(
        api='/collections/{collection_name}',
        method="GET",
        path_params={'collection_name': collection_name},
    )
    assert response.ok

    # Create nested index
    response = request_with_validation(
        api='/collections/{collection_name}/index',
        method="PUT",
        path_params={'collection_name': collection_name},
        query_params={'wait': 'true'},
        body={
            "field_name": "country.capital",
            "field_schema": "keyword"
        }
    )
    assert response.ok

    # Create nested array index
    response = request_with_validation(
        api='/collections/{collection_name}/index',
        method="PUT",
        path_params={'collection_name': collection_name},
        query_params={'wait': 'true'},
        body={
            "field_name": "country.cities[].population",
            "field_schema": "float"
        }
    )
    assert response.ok

    # Validate index creation
    response = request_with_validation(
        api='/collections/{collection_name}',
        method="GET",
        path_params={'collection_name': collection_name},
    )
    assert response.ok
    assert response.json()['result']['payload_schema']['country.capital']['data_type'] == "keyword"
    assert response.json()['result']['payload_schema']['country.capital']['points'] == 4
    assert response.json()['result']['payload_schema']['country.cities[].population']['data_type'] == "float"
    assert response.json()['result']['payload_schema']['country.cities[].population']['points'] == 4 # indexed records

    # Search nested through with payload index
    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "country.capital",
                        "match": {
                            "value": "London"
                        }
                    }
                ]
            },
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 1
    assert response.json()['result']['points'][0]['payload']['country']['name'] == "England"

    # Search nested without payload index
    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "country.name",
                        "match": {
                            "value": "France"
                        }
                    }
                ]
            },
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 1
    assert response.json()['result']['points'][0]['payload']['country']['capital'] == "Paris"

    # Search through array without payload index
    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "country.cities.population", # Do not implicitly do inside nested array
                        "range": {
                            "gte": 9.0,
                        }
                    }
                ]
            },
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 0

    # Search through array with payload index
    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "country.cities[].population",
                        "range": {
                            "gte": 9.0,
                        }
                    }
                ]
            },
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 1
    # Only Japan has a city with population greater than 9.0
    assert response.json()['result']['points'][0]['payload']['country']['name'] == "Japan"

    # Search through array without payload index
    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "country.cities[].sightseeing",
                        "match": {
                            "value": "Eiffel Tower"
                        }
                    }
                ]
            },
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 1
    assert response.json()['result']['points'][0]['payload']['country']['capital'] == "Paris"

    # Delete indexes
    response = request_with_validation(
        api='/collections/{collection_name}/index/{field_name}',
        method="DELETE",
        path_params={'collection_name': collection_name, 'field_name': 'country.capital'},
        query_params={'wait': 'true'},
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}/index/{field_name}',
        method="DELETE",
        path_params={'collection_name': collection_name, 'field_name': 'country.cities[].population'},
        query_params={'wait': 'true'},
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}',
        method="GET",
        path_params={'collection_name': collection_name},
    )
    assert response.ok
    assert len(response.json()['result']['payload_schema']) == 0

