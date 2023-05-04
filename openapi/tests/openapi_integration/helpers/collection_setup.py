from .helpers import request_with_validation


def drop_collection(collection_name='test_collection'):
    response = request_with_validation(
        api='/collections/{collection_name}',
        method="DELETE",
        path_params={'collection_name': collection_name},
    )
    assert response.ok


def basic_collection_setup(collection_name='test_collection', on_disk_payload=False):
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
                    "payload": {"city": "Berlin"}
                },
                {
                    "id": 2,
                    "vector": [0.19, 0.81, 0.75, 0.11],
                    "payload": {"city": ["Berlin", "London"]}
                },
                {
                    "id": 3,
                    "vector": [0.36, 0.55, 0.47, 0.94],
                    "payload": {"city": ["Berlin", "Moscow"]}
                },
                {
                    "id": 4,
                    "vector": [0.18, 0.01, 0.85, 0.80],
                    "payload": {"city": ["London", "Moscow"]}
                },
                {
                    "id": 5,
                    "vector": [0.24, 0.18, 0.22, 0.44],
                    "payload": {"count": 0}
                },
                {
                    "id": 6,
                    "vector": [0.35, 0.08, 0.11, 0.44]
                },
                {
                    "id": 7,
                    "vector": [0.25, 0.98, 0.14, 0.43],
                    "payload": {"city": None}
                },
                {
                    "id": 8,
                    "vector": [0.79, 0.53, 0.72, 0.15],
                    "payload": {"city": []}
                },
            ]
        }
    )
    assert response.ok


def multivec_collection_setup(collection_name='test_collection', on_disk_payload=False):
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
                "image": {
                    "size": 4,
                    "distance": "Dot"
                },
                "text": {
                    "size": 8,
                    "distance": "Cosine"
                }
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
                    "vector": {
                        "image": [0.05, 0.61, 0.76, 0.74],
                        "text": [0.05, 0.61, 0.76, 0.74, 0.05, 0.61, 0.76, 0.74],
                    },
                    "payload": {"city": "Berlin"}
                },
                {
                    "id": 2,
                    "vector": {
                        "image": [0.19, 0.81, 0.75, 0.11],
                        "text": [0.19, 0.81, 0.75, 0.11, 0.19, 0.81, 0.75, 0.11],
                    },
                    "payload": {"city": ["Berlin", "London"]}
                },
                {
                    "id": 3,
                    "vector": {
                        "image": [0.36, 0.55, 0.47, 0.94],
                        "text": [0.36, 0.55, 0.47, 0.94, 0.36, 0.55, 0.47, 0.94],
                    },
                    "payload": {"city": ["Berlin", "Moscow"]}
                },
                {
                    "id": 4,
                    "vector": {
                        "image": [0.18, 0.01, 0.85, 0.80],
                        "text": [0.18, 0.01, 0.85, 0.80, 0.18, 0.01, 0.85, 0.80],
                    },
                    "payload": {"city": ["London", "Moscow"]}
                },
                {
                    "id": 5,
                    "vector": {
                        "image": [0.24, 0.18, 0.22, 0.44],
                        "text": [0.24, 0.18, 0.22, 0.44, 0.24, 0.18, 0.22, 0.44],
                    },
                    "payload": {"count": 0}
                },
                {
                    "id": 6,
                    "vector": {
                        "image": [0.35, 0.08, 0.11, 0.44],
                        "text": [0.35, 0.08, 0.11, 0.44, 0.35, 0.08, 0.11, 0.44],
                    }
                }
            ]
        }
    )
    assert response.ok
