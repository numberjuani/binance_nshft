# binance_not_so_high_freq_trader
1. Streams orderbook and trades data from binance API, stores it in memory.
2. Trains xgboost multiclass classification, saves the data as dmats in a S3 storage bucket, and the model locally.
3. Uses the model to predict the next price movement, and places orders accordingly.
## Needs:
1. A binance API key and secret.
2. A S3 bucket.
3. Order/position management.
4. Docker build
5. Setup  `Deploy to ECS` action, available in github actions. https://github.com/marketplace/actions/amazon-ecs-deploy-task-definition-action-for-github-actions
