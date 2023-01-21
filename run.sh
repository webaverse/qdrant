#!/bin/bash

# sudo sh -c "ulimit -n 65535"
sudo docker run -p 6333:6333 \
  -v $(pwd)/data/storage:/qdrant/storage \
  -v $(pwd)/data/custom_config.yaml:/qdrant/config/production.yaml \
  --ulimit nofile=65535:65535 \
  qdrant/qdrant
