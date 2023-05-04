import pytest

from .helpers.collection_setup import basic_collection_setup, drop_collection
from .helpers.helpers import request_with_validation

collection_name = 'test_collection'


@pytest.fixture(autouse=True)
def setup():
    basic_collection_setup(collection_name=collection_name)
    yield
    drop_collection(collection_name=collection_name)


def test_points_retrieve():
    points_retrieve()


def points_retrieve():
    response = request_with_validation(
        api='/collections/{collection_name}/points/{id}',
        method="GET",
        path_params={'collection_name': collection_name, 'id': 2},
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}/points',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "ids": [1, 2]
        }
    )
    assert response.ok
    assert len(response.json()['result']) == 2

    response = request_with_validation(
        api='/collections/{collection_name}',
        method="GET",
        path_params={'collection_name': collection_name},
    )
    assert response.ok
    assert response.json()['result']['vectors_count'] == 8

    response = request_with_validation(
        api='/collections/{collection_name}/points/search',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "vector": [0.2, 0.1, 0.9, 0.7],
            "limit": 3
        }
    )
    assert response.ok
    assert len(response.json()['result']) == 3

    response = request_with_validation(
        api='/collections/{collection_name}/points/search',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "filter": {
                "should": [
                    {
                        "key": "city",
                        "match": {
                            "value": "London"
                        }
                    }
                ]
            },
            "vector": [0.2, 0.1, 0.9, 0.7],
            "limit": 3
        }
    )
    assert response.ok
    # only 2 London records in collection
    assert len(response.json()['result']) == 2

    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={"offset": 2, "limit": 2, "with_vector": True}
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 2


def test_exclude_payload():
    exclude_payload()


def exclude_payload():
    response = request_with_validation(
        api='/collections/{collection_name}/points/search',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "vector": [0.2, 0.1, 0.9, 0.7],
            "limit": 5,
            "filter": {
                "should": [
                    {
                        "key": "city",
                        "match": {
                            "value": "London"
                        }
                    }
                ]
            },
            "with_payload": {
                "exclude": ["city"]
            }
        }
    )
    assert response.ok
    assert len(response.json()['result']) > 0
    for result in response.json()['result']:
        assert 'city' not in result['payload']


def test_is_empty_condition():
    is_empty_condition()


def is_empty_condition():
    response = request_with_validation(
        api='/collections/{collection_name}/points/search',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "vector": [0.2, 0.1, 0.9, 0.7],
            "limit": 5,
            "filter": {
                "should": [
                    {
                        "is_empty": {
                            "key": "city"
                        }
                    }
                ]
            },
            "with_payload": True
        }
    )

    assert response.ok

    json = response.json()
    assert len(json['result']) == 4

    ids = [x['id'] for x in json['result']]
    assert 5 in ids
    assert 6 in ids
    assert 7 in ids
    assert 8 in ids


def test_is_null_condition():
    is_null_condition()


def is_null_condition():
    response = request_with_validation(
        api='/collections/{collection_name}/points/search',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "vector": [0.2, 0.1, 0.9, 0.7],
            "limit": 5,
            "filter": {
                "should": [
                    {
                        "is_null": {
                            "key": "city"
                        }
                    }
                ]
            },
            "with_payload": True
        }
    )
    assert response.ok

    json = response.json()
    assert len(json['result']) == 1

    ids = [x['id'] for x in json['result']]
    assert 7 in ids


def test_recommendation():
    recommendation()


def recommendation():
    response = request_with_validation(
        api='/collections/{collection_name}/points/recommend',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "limit": 3,
            "negative": [],
            "positive": [1],
            "with_vector": False,
            "with_payload": True
        }
    )
    assert len(response.json()['result']) == 3
    assert response.json()['result'][0]['payload'] is not None
    assert response.ok


def test_query_nested():
    query_nested()


def query_nested():
    response = request_with_validation(
        api='/collections/{collection_name}/points',
        method="PUT",
        path_params={'collection_name': collection_name},
        query_params={'wait': 'true'},
        body={
            "points": [
                {
                    "id": 8,
                    "vector": [0.15, 0.31, 0.76, 0.74],
                    "payload": {
                        "database_id": {
                            "type": "keyword",
                            "value": "8594ff5d-265f-4785-a9f5-b3b4b9665506"
                        }
                    }
                }
            ]
        }
    )
    assert response.ok

    response = request_with_validation(
        api='/collections/{collection_name}/points/scroll',
        method="POST",
        path_params={'collection_name': collection_name},
        body={
            "offset": None,
            "limit": 10,
            "with_vector": True,
            "filter": {
                "must": [
                    {
                        "key": "database_id.value",
                        "match": {
                            "value": "8594ff5d-265f-4785-a9f5-b3b4b9665506"
                        }
                    }
                ]
            }
        }
    )
    assert response.ok
    assert len(response.json()['result']['points']) == 1


def test_with_vectors_alias_of_with_vector():
    database_id = "8594ff5d-265f-adfh-a9f5-b3b4b9665506"
    vector = [0.15, 0.31, 0.76, 0.74]
    
    response = request_with_validation(
        api='/collections/{collection_name}/points',
        method="PUT",
        path_params={'collection_name': collection_name},
        query_params={'wait': 'true'},
        body={
            "points": [
                {
                    "id": 8,
                    "vector": vector,
                    "payload": {
                        "database_id": database_id,
                    }
                }
            ]
        }
    )
    assert response.ok
        
    def scroll_with_vector(keyword):
        response = request_with_validation(
            api='/collections/{collection_name}/points/scroll',
            method='POST',
            path_params={'collection_name': collection_name},
            body={
                keyword: True, # <--- should make no difference
                "filter": {
                    "must": [
                        {
                            "key": "database_id",
                            "match": {
                                "value": database_id
                            }
                        }
                    ]
                },
                "limit": 1,
            }
        )
        
        assert response.ok
        body = response.json()
        
        assert body["result"]["points"][0]["vector"] == vector
        
    scroll_with_vector("with_vector")
    scroll_with_vector("with_vectors")