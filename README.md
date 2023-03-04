# binance_data_gatherer
1. Streams orderbook data from Binance websocket API, and verifies that the state matches the REST API at an update id height.
2. Creates a CSV file with orderbook data, compresses it, and upload it to S3 bucket.