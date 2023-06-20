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

At the top of the configuration file, you specify the *hierarchy* of the entries and 
the *count* for each entry. The entries in the *hierarchy* **MUST** be the same as a 
subset of the specified object class descriptions, otherwise the hierarchy cannot be built.
Equally, the length of the *count*-array **MUST** be equal to the length of the hierarchy.

The *count* array specifies the amount of entries for the object class at the same index.
The *count* at any index specifies the number of entries *per entry* of the previous level.
This means that a "count-hierarchy" of [2, 5] will create 12 entries: 2 at the top level and 
5 for each sublevel. Keep that in mind when creating entries.

For example, to generate inetOrgPerson entries, the config would look like this:

```
[inetOrgPerson]
rdn="uid"
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

"rdn" is a special attribute that MUST be present and MUST be the name of another attribute present
for the object class. It will be used as the RDN value for the generated entry.

*Note*: At the moment, the entries are "inconsistent", meaning that reusing the same file 
for an entry in multiple attributes will yield different results. This shouldn't matter
as the sole purpose of this program is to generate random data, but just in case: Do not 
rely on assumptions, such as "cn = firstname lastname", thus "givenName = firstname". It is 
much more likely that "givenName = firstname'", where firstname' MIGHT be the same, but 
probably isn't.

## Modifiers
To allow reusing text files, some modifiers can be applied to the configuration values. At the 
time of writing, there are the following modifiers:

* file(arg) - Use a random line of this file as a value, whenever the modifier is used

* combine(args...) - Combines multiple values. If a provided argument is a file, a random value 
from the file will be used. If it cannot be resolved to a file, the value will be used as-is.

* lowercase(arg) - Will transform the value or result of previous modifiers into lowercase

* uppercase(arg) - Will transform the value or result of previous modifiers into uppercase

# Generated Output
When using `export`, `ldapfill` will generate LDIF-Files containing the generated entries, using the provided
base-dn. Additionally, it is possible to export the generated ldif as CSV, allowing you to use the 
entries, for example, with JMeter.
