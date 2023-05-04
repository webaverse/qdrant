import json
import os
import shutil
from subprocess import Popen
import time
from typing import Tuple, Callable, Dict, List, Optional
import requests
import socket
from contextlib import closing
from pathlib import Path
import pytest
from .assertions import assert_http_ok

# Tracks processes that need to be killed at the end of the test
processes = []


@pytest.fixture(autouse=True)
def every_test():
    yield
    print()
    while len(processes) > 0:
        p = processes.pop(0)
        print(f"Killing {p.pid}")
        p.kill()


def get_port() -> int:
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(('', 0))
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return s.getsockname()[1]


def get_env(p2p_port: int, grpc_port: int, http_port: int) -> Dict[str, str]:
    env = os.environ.copy()
    env["QDRANT__CLUSTER__ENABLED"] = "true"
    env["QDRANT__CLUSTER__P2P__PORT"] = str(p2p_port)
    env["QDRANT__SERVICE__HTTP_PORT"] = str(http_port)
    env["QDRANT__SERVICE__GRPC_PORT"] = str(grpc_port)
    env["QDRANT__LOG_LEVEL"] = "DEBUG,raft::raft=info"
    return env


def get_uri(port: int) -> str:
    return f"http://127.0.0.1:{port}"


def assert_project_root():
    directory_path = os.getcwd()
    folder_name = os.path.basename(directory_path)
    assert folder_name == "qdrant"


def get_qdrant_exec() -> str:
    directory_path = os.getcwd()
    qdrant_exec = directory_path + "/target/debug/qdrant"
    return qdrant_exec


def get_pytest_current_test_name() -> str:
    # https://docs.pytest.org/en/latest/example/simple.html#pytest-current-test-environment-variable
    return os.environ.get('PYTEST_CURRENT_TEST').split(':')[-1].split(' ')[0]


def init_pytest_log_folder() -> str:
    test_name = get_pytest_current_test_name()
    log_folder = f"consensus_test_logs/{test_name}"
    if not os.path.exists(log_folder):
        os.makedirs(log_folder)
    return log_folder


# Starts a peer and returns its api_uri
def start_peer(peer_dir: Path, log_file: str, bootstrap_uri: str, port=None, extra_env=None) -> str:
    if extra_env is None:
        extra_env = {}
    p2p_port = get_port() if port is None else port + 0
    grpc_port = get_port() if port is None else port + 1
    http_port = get_port() if port is None else port + 2
    env = {
        **get_env(p2p_port, grpc_port, http_port),
        **extra_env
    }
    test_log_folder = init_pytest_log_folder()
    log_file = open(f"{test_log_folder}/{log_file}", "w")
    print(f"Starting follower peer with bootstrap uri {bootstrap_uri},"
          f" http: http://localhost:{http_port}/cluster, p2p: {p2p_port}")

    this_peer_consensus_uri = get_uri(p2p_port)
    processes.append(
        Popen([get_qdrant_exec(), "--bootstrap", bootstrap_uri, "--uri", this_peer_consensus_uri], env=env,
              cwd=peer_dir, stderr=log_file))
    return get_uri(http_port)


# Starts a peer and returns its api_uri and p2p_uri
def start_first_peer(peer_dir: Path, log_file: str, port=None) -> Tuple[str, str]:
    p2p_port = get_port() if port is None else port + 0
    grpc_port = get_port() if port is None else port + 1
    http_port = get_port() if port is None else port + 2
    env = get_env(p2p_port, grpc_port, http_port)
    test_log_folder = init_pytest_log_folder()
    log_file = open(f"{test_log_folder}/{log_file}", "w")
    bootstrap_uri = get_uri(p2p_port)
    print(f"\nStarting first peer with uri {bootstrap_uri},"
          f" http: http://localhost:{http_port}/cluster, p2p: {p2p_port}")
    processes.append(
        Popen([get_qdrant_exec(), "--uri", bootstrap_uri], env=env, cwd=peer_dir, stderr=log_file))
    return get_uri(http_port), bootstrap_uri


def start_cluster(tmp_path, num_peers, port_seed=None):
    assert_project_root()
    peer_dirs = make_peer_folders(tmp_path, num_peers)

    # Gathers REST API uris
    peer_api_uris = []

    # Start bootstrap
    (bootstrap_api_uri, bootstrap_uri) = start_first_peer(peer_dirs[0], "peer_0_0.log", port=port_seed)
    peer_api_uris.append(bootstrap_api_uri)

    # Wait for leader
    leader = wait_peer_added(bootstrap_api_uri)

    port = None
    # Start other peers
    for i in range(1, len(peer_dirs)):
        if port_seed is not None:
            port = port_seed + i * 100
        peer_api_uris.append(start_peer(peer_dirs[i], f"peer_0_{i}.log", bootstrap_uri, port=port))

    # Wait for cluster
    wait_for_uniform_cluster_status(peer_api_uris, leader)

    return peer_api_uris, peer_dirs, bootstrap_uri


def make_peer_folder(base_path: Path, peer_number: int) -> Path:
    peer_dir = base_path / f"peer{peer_number}"
    peer_dir.mkdir()
    shutil.copytree("config", peer_dir / "config")
    return peer_dir


def make_peer_folders(base_path: Path, n_peers: int) -> List[Path]:
    peer_dirs = []
    for i in range(n_peers):
        peer_dir = make_peer_folder(base_path, i)
        peer_dirs.append(peer_dir)
    return peer_dirs


def get_cluster_info(peer_api_uri: str) -> dict:
    r = requests.get(f"{peer_api_uri}/cluster")
    assert_http_ok(r)
    res = r.json()["result"]
    return res


def print_clusters_info(peer_api_uris: [str]):
    for uri in peer_api_uris:
        try:
            # do not crash if the peer is not online
            print(json.dumps(get_cluster_info(uri), indent=4))
        except requests.exceptions.ConnectionError:
            print(f"Can't retrieve cluster info for offline peer {uri}")


def fetch_highest_peer_id(peer_api_uris: [str]) -> str:
    max_peer_id = 0
    max_peer_url = None
    for uri in peer_api_uris:
        try:
            # do not crash if the peer is not online
            peer_id = get_cluster_info(uri)['peer_id']
            if peer_id > max_peer_id:
                max_peer_id = peer_id
                max_peer_url = uri
        except requests.exceptions.ConnectionError:
            print(f"Can't retrieve cluster info for offline peer {uri}")
    return max_peer_url


def get_collection_cluster_info(peer_api_uri: str, collection_name: str) -> dict:
    r = requests.get(f"{peer_api_uri}/collections/{collection_name}/cluster")
    assert_http_ok(r)
    res = r.json()["result"]
    return res


def get_collection_info(peer_api_uri: str, collection_name: str) -> dict:
    r = requests.get(f"{peer_api_uri}/collections/{collection_name}")
    assert_http_ok(r)
    res = r.json()["result"]
    return res


def print_collection_cluster_info(peer_api_uri: str, collection_name: str):
    print(json.dumps(get_collection_cluster_info(peer_api_uri, collection_name), indent=4))


def get_leader(peer_api_uri: str) -> str:
    r = requests.get(f"{peer_api_uri}/cluster")
    assert_http_ok(r)
    return r.json()["result"]["raft_info"]["leader"]


def check_leader(peer_api_uri: str, expected_leader: str) -> bool:
    try:
        r = requests.get(f"{peer_api_uri}/cluster")
        assert_http_ok(r)
        leader = r.json()["result"]["raft_info"]["leader"]
        correct_leader = leader == expected_leader
        if not correct_leader:
            print(f"Cluster leader invalid for peer {peer_api_uri} {leader}/{expected_leader}")
        return correct_leader
    except requests.exceptions.ConnectionError:
        # the api is not yet available - caller needs to retry
        print(f"Could not contact peer {peer_api_uri} to fetch cluster leader")
        return False


def leader_is_defined(peer_api_uri: str) -> bool:
    try:
        r = requests.get(f"{peer_api_uri}/cluster")
        assert_http_ok(r)
        leader = r.json()["result"]["raft_info"]["leader"]
        return leader is not None
    except requests.exceptions.ConnectionError:
        # the api is not yet available - caller needs to retry
        print(f"Could not contact peer {peer_api_uri} to fetch leader info")
        return False


def check_cluster_size(peer_api_uri: str, expected_size: int) -> bool:
    try:
        r = requests.get(f"{peer_api_uri}/cluster")
        assert_http_ok(r)
        peers = r.json()["result"]["peers"]
        correct_size = len(peers) == expected_size
        if not correct_size:
            print(f"Cluster size invalid for peer {peer_api_uri} {len(peers)}/{expected_size}")
        return correct_size
    except requests.exceptions.ConnectionError:
        # the api is not yet available - caller needs to retry
        print(f"Could not contact peer {peer_api_uri} to fetch cluster size")
        return False


def all_nodes_cluster_info_consistent(peer_api_uris: [str], expected_leader: str) -> bool:
    expected_size = len(peer_api_uris)
    for uri in peer_api_uris:
        if check_leader(uri, expected_leader) and check_cluster_size(uri, expected_size):
            continue
        else:
            return False
    return True


def all_nodes_respond(peer_api_uris: [str]) -> bool:
    for uri in peer_api_uris:
        try:
            r = requests.get(f"{uri}/collections")
            assert_http_ok(r)
        except requests.exceptions.ConnectionError:
            print(f"Could not contact peer {uri} to fetch collections")
            return False
    return True


def collection_exists_on_all_peers(collection_name: str, peer_api_uris: [str]) -> bool:
    for uri in peer_api_uris:
        r = requests.get(f"{uri}/collections")
        assert_http_ok(r)
        collections = r.json()["result"]["collections"]
        filtered_collections = [c for c in collections if c['name'] == collection_name]
        if len(filtered_collections) == 0:
            print(
                f"Collection '{collection_name}' does not exist on peer {uri} found {json.dumps(collections, indent=4)}")
            return False
        else:
            continue
    return True


def check_collection_local_shards_count(peer_api_uri: str, collection_name: str,
                                        expected_local_shard_count: int) -> bool:
    collection_cluster_info = get_collection_cluster_info(peer_api_uri, collection_name)
    local_shard_count = len(collection_cluster_info["local_shards"])
    return local_shard_count == expected_local_shard_count


def check_collection_shard_transfers_count(peer_api_uri: str, collection_name: str,
                                           expected_shard_transfers_count: int) -> bool:
    collection_cluster_info = get_collection_cluster_info(peer_api_uri, collection_name)
    local_shard_count = len(collection_cluster_info["shard_transfers"])
    return local_shard_count == expected_shard_transfers_count


def check_all_replicas_active(peer_api_uri: str, collection_name: str) -> bool:
    collection_cluster_info = get_collection_cluster_info(peer_api_uri, collection_name)
    for shard in collection_cluster_info["local_shards"]:
        if shard['state'] != 'Active':
            return False
    for shard in collection_cluster_info["remote_shards"]:
        if shard['state'] != 'Active':
            return False
    return True


def check_some_replicas_not_active(peer_api_uri: str, collection_name: str) -> bool:
    return not check_all_replicas_active(peer_api_uri, collection_name)


def check_collection_cluster(peer_url, collection_name):
    res = requests.get(f"{peer_url}/collections/{collection_name}/cluster", timeout=10)
    assert_http_ok(res)
    return res.json()["result"]['local_shards'][0]


WAIT_TIME_SEC = 30
RETRY_INTERVAL_SEC = 0.5


def wait_peer_added(peer_api_uri: str, expected_size: int = 1) -> str:
    wait_for(check_cluster_size, peer_api_uri, expected_size)
    wait_for(leader_is_defined, peer_api_uri)
    return get_leader(peer_api_uri)


def wait_for_some_replicas_not_active(peer_api_uri: str, collection_name: str):
    try:
        wait_for(check_some_replicas_not_active, peer_api_uri, collection_name)
    except Exception as e:
        print_clusters_info([peer_api_uri])
        raise e


def wait_for_all_replicas_active(peer_api_uri: str, collection_name: str):
    try:
        wait_for(check_all_replicas_active, peer_api_uri, collection_name)
    except Exception as e:
        print_clusters_info([peer_api_uri])
        raise e


def wait_for_uniform_cluster_status(peer_api_uris: [str], expected_leader: str):
    try:
        wait_for(all_nodes_cluster_info_consistent, peer_api_uris, expected_leader)
    except Exception as e:
        print_clusters_info(peer_api_uris)
        raise e


def wait_all_peers_up(peer_api_uris: [str]):
    try:
        wait_for(all_nodes_respond, peer_api_uris)
    except Exception as e:
        print_clusters_info(peer_api_uris)
        raise e


def wait_for_uniform_collection_existence(collection_name: str, peer_api_uris: [str]):
    try:
        wait_for(collection_exists_on_all_peers, collection_name, peer_api_uris)
    except Exception as e:
        print_clusters_info(peer_api_uris)
        raise e


def wait_for_collection_shard_transfers_count(peer_api_uri: str, collection_name: str,
                                              expected_shard_transfer_count: int):
    try:
        wait_for(check_collection_shard_transfers_count, peer_api_uri, collection_name, expected_shard_transfer_count)
    except Exception as e:
        print_collection_cluster_info(peer_api_uri, collection_name)
        raise e


def wait_for_collection_local_shards_count(peer_api_uri: str, collection_name: str, expected_local_shard_count: int):
    try:
        wait_for(check_collection_local_shards_count, peer_api_uri, collection_name, expected_local_shard_count)
    except Exception as e:
        print_collection_cluster_info(peer_api_uri, collection_name)
        raise e


def wait_for(condition: Callable[..., bool], *args):
    start = time.time()
    while not condition(*args):
        elapsed = time.time() - start
        if elapsed > WAIT_TIME_SEC:
            raise Exception(
                f"Timeout waiting for condition {condition.__name__} to be satisfied in {WAIT_TIME_SEC} seconds")
        else:
            time.sleep(RETRY_INTERVAL_SEC)


def peer_is_online(peer_api_uri: str) -> bool:
    try:
        r = requests.get(f"{peer_api_uri}")
        return r.status_code == 200
    except:
        return False


def wait_for_peer_online(peer_api_uri: str):
    try:
        wait_for(peer_is_online, peer_api_uri)
    except Exception as e:
        print_clusters_info([peer_api_uri])
        raise e


def check_collection_size(peer_api_uri: str, collection_name: str, expected_size: int) -> bool:
    collection_cluster_info = get_collection_info(peer_api_uri, collection_name)
    return collection_cluster_info['points_count'] == expected_size


def wait_collection_size(peer_api_uri: str, collection_name: str, expected_size: int):
    try:
        wait_for(check_collection_size, peer_api_uri, collection_name, expected_size)
    except Exception as e:
        print_collection_cluster_info(peer_api_uri, collection_name)
        raise e


def wait_collection_on_all_peers(collection_name: str, peer_api_uris: [str], max_wait=30):
    # Check that it exists on all peers
    while True:
        exists = True
        for url in peer_api_uris:
            r = requests.get(f"{url}/collections")
            assert_http_ok(r)
            collections = r.json()["result"]["collections"]
            exists &= any(collection["name"] == collection_name for collection in collections)
        if exists:
            break
        else:
            # Wait until collection is created on all peers
            # Consensus guarantees that collection will appear on majority of peers, but not on all of them
            # So we need to wait a bit extra time
            time.sleep(1)
            max_wait -= 1
        if max_wait <= 0:
            raise Exception("Collection was not created on all peers in time")


def wait_collection_exists_and_active_on_all_peers(collection_name: str, peer_api_uris: [str], max_wait=30):
    wait_collection_on_all_peers(collection_name, peer_api_uris, max_wait)
    for peer_uri in peer_api_uris:
        # Collection is active on all peers
        wait_for_all_replicas_active(collection_name=collection_name, peer_api_uri=peer_uri)
