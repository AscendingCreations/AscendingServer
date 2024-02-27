# AscendingServer
Open Source Game Server written in rust. Part of Ascending Source


### Generate TLS Keys for client and Server.

Server needs client.crt and client.key
Client just needs ca.pem

Run this command to Create the Certificate Authority
```openssl req -new -newkey rsa:4096 -nodes -out ca.csr -keyout ca.key```

it will display the below Messages you can set it up similair to how I have it.
This file will be used to sign all Certificates so we know they are legitly coming
from the Server. You should never give the ca.key with your server or client since if anyone
gets this key your entire cert is null void.

```
If you enter '.', the field will be left blank.
-----
Country Name (2 letter code) [AU]:US
State or Province Name (full name) [Some-State]:Michigan
Locality Name (eg, city) []:Mendon
Organization Name (eg, company) [Internet Widgits Pty Ltd]:Ascending Creations
Organizational Unit Name (eg, section) []:
Common Name (e.g. server FQDN or YOUR name) []:genusis
Email Address []:genusistimelord@outlook.com

Please enter the following 'extra' attributes
to be sent with your certificate request
A challenge password []:****************
An optional company name []:
```


Then we run this command to Create the Public Authenticator file to use with out Client and server.
```openssl x509 -trustout -signkey ca.key -days 365 -req -in ca.csr -out ca.pem```

After which we need to create the clients pub and private keys and cert. First we make the Clients Private key
```openssl genrsa -out client.key 4096```

Then we make the Clients csr is a certificate request to the ca to generate our signed cer.
this is only used to make the cer and can be discarded afterwards.
```openssl req -new -key client.key -out client.csr```

then we will set the csr to are ca and generate the Cer that goes to the Server. The server will send this cert to the 
client and the client can verify it came from the server using the ca.pem
```openssl ca -in client.csr -out client.cer```

If you are having errors due to the openssl.cnf on windows then you just need to update the cnf to point to the correct directory you placed your 
ca.pem and ca.key in like the below

```
[ CA_default ]

dir		= C:/Sources		# Where everything is kept
certs		= $dir/certs		# Where the issued certs are kept
crl_dir		= $dir/crl		# Where the issued crl are kept
database	= $dir/index.txt	# database index file.
#unique_subject	= no			# Set to 'no' to allow creation of
					# several certs with same subject.
new_certs_dir	= $dir/newcerts		# default place for new certs.

certificate	= $dir/ca.pem 	# The CA certificate
serial		= $dir/serial 		# The current serial number
crlnumber	= $dir/crlnumber	# the current crl number
					# must be commented out to leave a V1 CRL
crl		= $dir/crl.pem 		# The current CRL
private_key	= $dir/ca.key # The private key
```

if you are getting a Error for a missing serial file then add a file named serial to the directory and insert the number 1000 into it.
once these are done the command should work.