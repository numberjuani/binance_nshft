# Binance "Not so high frequency" Trader
1. Connects to socket
2. Streams trades and orderbooks
3. Creates features from trades and orderbooks
4. Trains a GBDT model from the data
5. Makes predictions
6. Keeps retraining the model every [n] minutes