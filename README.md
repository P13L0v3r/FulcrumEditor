# Fulcrum
 
An object-oriented text editor to boost workflow for authors.

## What is Fulcrum?

Fulcrum is an object-oriented text editor that allows the creation of objects made up of properties. Writers can give an object any number of properties with any value, and later reference those properties in their writing to be able to easily change certain aspects of their writing without spamming Find/Replace. This lets authors get their ideas onto the page without having to worry about the pain of rewriting.

## Using Fulcrum

To create objects, use the following syntax.

    def object_name {
        first_property_name: "first value",
        second_property_name: "second value"
    }

Object and property names cannot contain any of the following characters: ,.<>!@#$%^&\*()\[\]|\\:;'"?/\`~+= or any whitespace character.

Any of these characters will be rendered after the value.

If you follow a value reference with a **!** or **^** then the value will be rendered with each word capitalized or the first word capitalized, respectively.

To reference an object and the value of one of its properties, use the following syntax.

    @object_name:first_property_name

## Example

### Input

    def character {
        name: "James",
        pronoun: "he",
        likes: "cake"
    }
    @character:name says that @character:pronoun likes @character:likes. @character:pronoun^ likes @character:likes so much!
    

### Output

> James says that he likes cake. He likes cake so much!

This demonstrates how an author could replace the name, pronouns, and likes of this character without having to dig through an entire manuscript to replace these things any time they're used.

## Object Bank

The Object Bank allows you to quickly reference your saved objects and their properties. Clicking an object's name will open a menu that contains that objects property names. Clicking one of these property names will insert the full reference path to that property.