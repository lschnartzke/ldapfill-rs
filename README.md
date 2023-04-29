# ldapfill

`ldapfill` is a small CLI tool that allows to generate LDAP directory content. It does this by
combining values from different files to build a valid ldap entries. It also allows you to
define the depth of the tree structure, to generate some form of distribution. The generated
content can be written directly to the server or exported into an ldif file.

The tool takes several text files as arguments, along with the login details for the server.
It then generates Entries and builds a tree-structure using these values. The files
are expecteded to be text files containing one "value" per line. The configuration for
the tool allows to specify which ldap fields should be filled with values from what file.

# Configuration
`ldapfill` can be configured using a standard TOML file. The contents of the config file describe
how individual LDAP entries are constructed.

For example, to generate inetOrgPerson entries, the config would look like this:

```
[inetOrgPerson]
cn=combine(file("firstname.txt"), " ", file("lastname.txt"))
givenName=file("firstname.txt")
sn=file("lastname.txt")
uid=lowercase(combine(file("firstname.txt"), " " file("lastname.txt"))
mail=lowercase(combine(file("firstname.txt") "." file("lastname.txt") "@" file("domains.txt"))
```

To explain:
The brackets define the objectClass of the entries.
On the left side of the equals (=) sign you specify attributes of the objectclass 
and on the right are some (optional) modifiers and the name of the file(s) from which the attribute 
values should be pulled.


## Modifiers
To allow reusing text files, some modifiers can be applied to the configuration values. At the 
time of writing, there are the following modifiers:

* file(arg) - Use a random line of this file as a value, whenever the modifier is used

* combine(args...) - Combines multiple values. If a provided argument is a file, a random value 
from the file will be used. If it cannot be resolved to a file, the value will be used as-is.

* lowercase(arg) - Will transform the value or result of previous modifiers into lowercase

* uppercase(arg) - Will transform the value or result of previous modifiers into uppercase

# Generated Output
`ldapfill` can create different kinds of output, depending on the parameters. Ideally, the output 
can be used for other tools, such as Jmeter, to perform benchmarks on servers without having to
manually create all tests and directory content from scratch.

Aside from the directory content itself, the tool can create search queries with a given filter 
template, (randomly chosen) search base and a given chance that the search will return no results.

The output for these queries will be stored in text files, using the chosen format (e.g. Jmeter test,
json, ldif, ...)
