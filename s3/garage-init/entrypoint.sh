NODE_ID=$(/garage node id 2>/dev/null | sed -n '1s/@.*//p')
RPC_HOST="$NODE_ID@garage:3901"
until /garage -h "$RPC_HOST" status >/dev/null 2>&1; do
  sleep 1
done
if [ -n "$NODE_ID" ]; then
  /garage -h "$RPC_HOST" layout assign -z dc1 -c 1G "$NODE_ID" || true
fi
/garage -h "$RPC_HOST" layout apply --version 1 || true
/garage -h "$RPC_HOST" bucket create data || true
/garage -h "$RPC_HOST" key import --yes -n minioadmin GKcafebabe00000000cafebabe cafebabe00000000cafebabe00000000cafebabe00000000cafebabe00000000 || true
/garage -h "$RPC_HOST" bucket allow --read --write --owner data --key minioadmin || true
echo "Garage initialized. S3 endpoint: http://localhost:9000"
