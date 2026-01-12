#!/bin/sh
# Default to 4 if nproc fails or returns 1 (though nproc usually works)
CORES=$(nproc) || CORES=4
echo "Starting $CORES workers..."

for i in $(seq 1 $CORES); do
  bun src/index.ts &
done

# Keep container running
wait
