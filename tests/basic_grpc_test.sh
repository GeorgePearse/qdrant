#!/bin/bash
# This test checks that Qdrant answers to all API mentioned in README.md as expected

set -ex

# Ensure current path is project root
cd "$(dirname "$0")/../"

QDRANT_HOST='localhost:6334'

docker_grpcurl="docker run --rm --network=host -v ${PWD}/lib/api/src/grpc/proto:/proto fullstorydev/grpcurl -plaintext -import-path /proto -proto qdrant.proto"

$docker_grpcurl -d '{
   "collection_name": "test_collection"
}' $QDRANT_HOST qdrant.Collections/Delete


$docker_grpcurl -d '{
   "collection_name": "test_collection",
   "vector_size": 4,
   "distance": "Dot"
}' $QDRANT_HOST qdrant.Collections/Create

$docker_grpcurl -d '{}' $QDRANT_HOST qdrant.Collections/List

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "wait": true,
  "points": [
    {
      "id": { "num": 1 },
      "vector": [0.05, 0.61, 0.76, 0.74],
      "payload": {
        "city": { "string_value": "Berlin" },
        "country":  { "string_value": "Germany" },
        "population": { "integer_value":  1000000 },
        "square": { "double_value": 12.5 },
        "coords": { "struct_value": { "fields": { "lat": { "double_value": 1.0 }, "lon": { "double_value": 2.0 } } } }
      }
    },
    {"id": { "num": 2 }, "vector": [0.19, 0.81, 0.75, 0.11], "payload": {"city": {"list_value": {"values": [{ "string_value": "Berlin" }, { "string_value": "London" }]}}}},
    {"id": { "num": 3 }, "vector": [0.36, 0.55, 0.47, 0.94], "payload": {"city": {"list_value": {"values": [{ "string_value": "Berlin" }, { "string_value": "Moscow" }]}}}},
    {"id": { "num": 4 }, "vector": [0.18, 0.01, 0.85, 0.80], "payload": {"city": {"list_value": {"values": [{ "string_value": "London" }, { "string_value": "Moscow" }]}}}},
    {"id": { "num": 5 }, "vector": [0.24, 0.18, 0.22, 0.44], "payload": {"count":{"list_value": {"values": [{ "integer_value": 0 }]}}}},
    {"id": { "num": 6 }, "vector": [0.35, 0.08, 0.11, 0.44]}
  ]
}' $QDRANT_HOST qdrant.Points/Upsert

$docker_grpcurl -d '{ "collection_name": "test_collection" }' $QDRANT_HOST qdrant.Collections/Get

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "vector": [0.2,0.1,0.9,0.7],
  "top": 3
}' $QDRANT_HOST qdrant.Points/Search

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "filter": {
    "should": [
      {
        "field": {
          "key": "city",
          "match": {
            "keyword": "London"
          }
        }
      }
    ]
  },
  "vector": [0.2,0.1,0.9,0.7],
  "top": 3
}' $QDRANT_HOST qdrant.Points/Search

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "limit": 2,
  "with_vector": true,
  "filter": {
    "should": [
      {
        "field": {
          "key": "city",
          "match": {
            "keyword": "London"
          }
        }
      }
    ]
  }
}' $QDRANT_HOST qdrant.Points/Scroll

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "with_vector": true,
  "ids": [{ "num": 2 }, { "num": 3 }, { "num": 4 }]
}' $QDRANT_HOST qdrant.Points/Get

$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "positive": [{ "num": 1 }],
  "negative": [{ "num": 2 }]
}' $QDRANT_HOST qdrant.Points/Recommend

# create alias
$docker_grpcurl -d '{
  "actions": [
    {
      "create_alias": {
        "alias_name": "test_alias",
        "collection_name": "test_collection"
      }
    }
  ]
}' $QDRANT_HOST qdrant.Collections/UpdateAliases

# search via alias
$docker_grpcurl -d '{
  "collection_name": "test_alias",
  "vector": [0.2,0.1,0.9,0.7],
  "top": 3
}' $QDRANT_HOST qdrant.Points/Search

# rename alias
$docker_grpcurl -d '{
  "actions": [
    {
      "rename_alias": {
        "old_alias_name": "test_alias",
        "new_alias_name": "new_test_alias"
      }
    }
  ]
}' $QDRANT_HOST qdrant.Collections/UpdateAliases

# search via renamed alias
$docker_grpcurl -d '{
  "collection_name": "new_test_alias",
  "vector": [0.2,0.1,0.9,0.7],
  "top": 3
}' $QDRANT_HOST qdrant.Points/Search

# delete alias
$docker_grpcurl -d '{
  "actions": [
    {
      "delete_alias": {
        "alias_name": "new_test_alias"
      }
    }
  ]
}' $QDRANT_HOST qdrant.Collections/UpdateAliases


$docker_grpcurl -d '{
  "collection_name": "test_collection",
  "with_vector": false,
  "with_payload": {
    "include": {"fields": ["population"]}
  },
  "ids": [{ "num": 1 }]
}' $QDRANT_HOST qdrant.Points/Get

#SAVED_VECTORS_COUNT=$(curl --fail -s "http://$QDRANT_HOST/collections/test_collection" | jq '.result.vectors_count')
#[[ "$SAVED_VECTORS_COUNT" == "6" ]] || {
#  echo 'check failed'
#  exit 1
#}
#
#curl -L -X POST "http://$QDRANT_HOST/collections/test_collection/points/search" \
#  -H 'Content-Type: application/json' \
#  --fail -s \
#  --data-raw '{
#        "vector": [0.2,0.1,0.9,0.7],
#        "top": 3
#    }' | jq
#
#curl -L -X POST "http://$QDRANT_HOST/collections/test_collection/points/search" \
#  --fail -s \
#  -H 'Content-Type: application/json' \
#  --data-raw '{
#      "filter": {
#          "should": [
#              {
#                  "key": "city",
#                  "match": {
#                      "keyword": "London"
#                  }
#              }
#          ]
#      },
#      "vector": [0.2, 0.1, 0.9, 0.7],
#      "top": 3
#  }' | jq