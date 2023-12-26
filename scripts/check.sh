#!/bin/bash

#set -ex
password=$1
salt=$2
#echo

# Get password hash from /etc/shadow
#salt=$(sudo grep "^$username:" /etc/shadow | cut -d ":" -f 2)

# Perl
result=$(perl -e "
use strict;
use warnings;

my \$passwd = shift;
my \$salt = shift;

# crypt
my \$encrypted_input = crypt(\$passwd, \$salt);
# Check if the encrypted input matches the stored hash
if (\$encrypted_input eq \$salt) {
    print \"ok\\n\";
} else {
    print \"fail\\n\";
}
" "$password" "$salt")

# Output result
echo $result
