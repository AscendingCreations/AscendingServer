# AscendingServer
Open Source Game Server written in rust. Part of Ascending Source


### Generate TLS Keys for client and Server.

Server needs server.crt, server-key.pem and ca-crt.pem.
Client needs client.crt, client-key.pem and ca-crt.pem.

If you are on windows you will need to install OpenSSL: https://wiki.openssl.org/index.php/Binaries
or ```winget install -e ShiningLight.OpenSSL```

Then check to ensure you have it installed correctly via 
```openssl version -a```

Run this command to Generate a Certificate Authority (CA) file to use to generate are crt's.
```openssl req -new -x509 -days 9999 -keyout ca-key.pem -out ca-crt.pem```

You’ll be asked to insert a CA password. Input a preferred password that you’ll remember.
You’ll be prompted to specify a CA Common Name. Insert that you prefer like root.localhost or ca.localhost.

Generate a Server Certificate
```openssl genrsa -out server-key.pem 4096```
```openssl req -new -key server-key.pem -out server-csr.pem```
You’ll be prompted to specify a CA Common Name. Insert that you prefer like localhost or server.localhost.
Optionally insert a challenge password

The client will need to verify the Common Name, so make sure you have a valid DNS name for this.
Now sign the certificate using the Certificate Authority
```echo 'subjectAltName = IP:127.0.0.1' > server-crt.ext```
```openssl x509 -req -days 365 -CA ca-crt.pem -CAkey ca-key.pem -CAcreateserial -in server-csr.pem -out server.crt -extfile server-crt.ext```

Now Check if the Cert is Valid by running
```openssl verify -CAfile ca-crt.pem server.crt```

Generate a Client Certificate
```openssl genrsa -out client-key.pem 4096```
```openssl req -new -key client-key.pem -out client-csr.pem```

You’ll be prompted to specify a CA Common Name. Insert that you prefer like client.localhost. The server should not verify this, since it should not do a reverse DNS lookup.
Optionally insert a challenge password

Now sign the certificate using the Certificate Authority
```echo 'subjectAltName = IP:127.0.0.1' > client-crt.ext```
```openssl x509 -req -days 365 -CA ca-crt.pem -CAkey ca-key.pem -CAcreateserial -in client-csr.pem -out client.crt -extfile client-crt.ext```

And then Verify the key
```openssl verify -CAfile ca-crt.pem client.crt```

These Steps are from https://medium.com/weekly-webtips/how-to-generate-keys-for-mutual-tls-authentication-a90f53bcec64
and will be hosted here just in case this site ever does die. 
