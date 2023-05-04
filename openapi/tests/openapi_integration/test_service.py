import pytest
from datetime import datetime, timedelta

from .helpers.helpers import request_with_validation
from .helpers.collection_setup import basic_collection_setup, drop_collection

collection_name = 'test_collection_telemetry'

@pytest.fixture(autouse=True)
def setup():
    basic_collection_setup(collection_name=collection_name)
    yield
    drop_collection(collection_name=collection_name)

def test_metrics():
    response = request_with_validation(
        api='/metrics',
        method="GET",
    )
    assert response.ok

    # Probe some strings that must exist in the metrics output
    assert '# HELP app_info information about qdrant server' in response.text
    assert '# TYPE app_info counter' in response.text
    assert 'app_info{name="qdrant",version="' in response.text
    assert 'collections_total ' in response.text

def test_telemetry():
    response = request_with_validation(
        api='/telemetry',
        method="GET",
    )
    
    assert response.ok
    
    result = response.json()['result']
    
    assert result['collections']['number_of_collections'] >= 1

    endpoint = result['requests']['rest']['responses']['PUT /collections/{name}/points']
    assert endpoint['200']['count'] > 0
    
    last_queried = endpoint['200']['last_responded']
    last_queried = datetime.strptime(last_queried, "%Y-%m-%dT%H:%M:%S.%f%z")
    # Assert today
    assert last_queried.date() == datetime.now().date()