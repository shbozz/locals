# Locals
The better way to communicate

features:
* P2P networking (using libP2P)
* Chatting in groups
* Encrypted messages
* Custom group names
* Usernames
* Each message has a Unique ID (with a hash)
* Messages sent in the chat are saved
* Usernames and their multiaddr are saved 

planned features:
* Saved encryption keys
* GUI in Flutter and GTK4 + Libadwaita
* A feed where posts get shared with friends
  * once a user receives a post, they can interact with it 
  * if the user interacts with a post, it will be shared 
  * posts can be saved

Notice: This code is mostly a POC, so the error handling is bad and not everything works exactly as intended