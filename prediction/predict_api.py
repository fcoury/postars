from flask import Flask, request, jsonify
import pickle
import os
from encrypt_decrypt import decrypt_file, encrypt_file

app = Flask(__name__)

# Decrypt the files before loading
decrypt_file('spam_classifier_model.pkl', os.environ['PICKLE_KEY'].encode())
decrypt_file('vectorizer.pkl', os.environ['PICKLE_KEY'].encode())

# Load the trained model and vectorizer
with open('spam_classifier_model.pkl', 'rb') as model_file:
    model = pickle.load(model_file)

with open('vectorizer.pkl', 'rb') as vectorizer_file:
    vectorizer = pickle.load(vectorizer_file)

# Encrypt the files after loading
encrypt_file('spam_classifier_model.pkl', os.environ['PICKLE_KEY'].encode())
encrypt_file('vectorizer.pkl', os.environ['PICKLE_KEY'].encode())

@app.route('/predict', methods=['POST'])
def predict():
    emails = request.json.get('emails', [])
    emails_vectorized = vectorizer.transform(emails)
    predictions = model.predict(emails_vectorized)
    return jsonify(predictions.tolist())

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
