<h1 align="center">
 AscendingServer
</h1>
Open Source Game Server written in rust. Part of Ascending Source.

# Help

If you need help with this library or have suggestions please go to our [Discord Group](https://discord.gg/gVXNDwpS3Z)

## Creating settings.toml
In order to use the sever you need to create a file called settings.toml and copy the contents of settings.toml.default to it. Then you can make any changes to the settings and they will not get overwritten by or saved to the repository.

## Generate TLS Keys for client and Server.

Server needs server.crt, server-key.pem and ca-crt.pem.
Client needs ca-crt.pem.

If you are on windows you will need to install OpenSSL: https://wiki.openssl.org/index.php/Binaries
or ```winget install -e ShiningLight.OpenSSL```

Then check to ensure you have it installed correctly via 
```openssl version -a```

Run this command to Generate a Certificate Authority (CA) file to use to generate are crt's.
```openssl req -new -x509 -days 9999 -keyout ca-key.pem -out ca-crt.pem```

You’ll be asked to insert a CA password. Input a preferred password that you’ll remember.
You’ll be prompted to specify a CA Common Name. Insert that you prefer like root.localhost or ca.localhost.
ca-crt.pem is the key that will be used on both Server and Client. This allows the Server to send keys to the
Client. this checks if the keys are legit and not fakes.

Generate a Server Certificate
```openssl genrsa -out server-key.pem 4096```
```openssl req -new -key server-key.pem -out server-csr.pem```
You’ll be prompted to specify a CA Common Name. Insert that you prefer like localhost or server.localhost.
Optionally insert a challenge password

The client will need to verify the Common Name, so make sure you have a valid DNS name for this.
Now sign the certificate using the Certificate Authority
```echo 'subjectAltName = IP:127.0.0.1, IP:12.0.0.1' > server-crt.ext```
```openssl x509 -req -days 365 -CA ca-crt.pem -CAkey ca-key.pem -CAcreateserial -in server-csr.pem -out server.crt -extfile server-crt.ext```

Now Check if the Cert is Valid by running
```openssl verify -CAfile ca-crt.pem server.crt```

Generate a Client Certificate. You only need a Client Certificate if you are going to make 1 cert per users client and use it to deturmine if that client 
can connect to the server or not by exluding it or including it. Otherwise you do not need to make client certs.
```openssl genrsa -out client-key.pem 4096```
```openssl req -new -key client-key.pem -out client-csr.pem```

You’ll be prompted to specify a CA Common Name. Insert that you prefer like client.localhost. The server should not verify this, since it should not do a reverse DNS lookup.
Optionally insert a challenge password

Now sign the certificate using the Certificate Authority changing the 12.0.0.1 ip address with the one your server will use.
```echo 'subjectAltName = IP:127.0.0.1, IP:12.0.0.1' > client-crt.ext```
```openssl x509 -req -days 365 -CA ca-crt.pem -CAkey ca-key.pem -CAcreateserial -in client-csr.pem -out client.crt -extfile client-crt.ext```

And then Verify the key
```openssl verify -CAfile ca-crt.pem client.crt```

These Steps are from https://medium.com/weekly-webtips/how-to-generate-keys-for-mutual-tls-authentication-a90f53bcec64
and will be hosted here just in case this site ever does die. 

## Ascending Source Links
[`Ascending Server`](https://github.com/AscendingCreations/AscendingServer)
[`Ascending Client`](https://github.com/AscendingCreations/AscendingClient)
[`Ascending Editors`](https://github.com/AscendingCreations/AscendingEditors)
[`Ascending Map Editor`](https://github.com/AscendingCreations/AscendingMapEditor)
