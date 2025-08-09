# Minimum implementation

In order to make SMTP workable, the following minimum implementation is required for all receivers:

## Commands

- HELO
- MAIL
- RCPT
- DATA
- RSET
- NOOP
- QUIT

# Specification of Commands

## HELLO (HELO)

This command is used to identify the sender-SMTP to the receiver-SMTP.  The argument field contains the host name of the sender-SMTP.

The receiver-SMTP identifies itself to the sender-SMTP in the connection greeting reply, and in the response to this command.

This command and an OK reply to it confirm that both the sender-SMTP and the receiver-SMTP are in the initial state, that is, there is no transaction in progress and all state tables and buffers are cleared.

## MAIL (MAIL)

This command is used to initiate a mail transaction in which the mail data is delivered to one or more mailboxes.  The argument field contains a reverse-path.

The reverse-path consists of an optional list of hosts and the sender mailbox.  When the list of hosts is present, it is a "reverse" source route and indicates that the mail was relayed through each host on the list (the first host in the list was the most recent relay).  This list is used as a source route to return non-delivery notices to the sender. As each relay host adds itself to the beginning of the list, it must use its name as known in the IPCE to which it is relaying the mail rather than the IPCE from which the mail came (if they are different).  In some types of error reporting messages (for example, undeliverable mail notifications) the reverse-path may be null (see Example 7).

This command clears the reverse-path buffer, the forward-path buffer, and the mail data buffer; and inserts the reverse-path information from this command into the reverse-path buffer.

## RECIPIENT (RCPT)

This command is used to identify an individual recipient of the mail data; multiple recipients are specified by multiple use of this command.

The forward-path consists of an optional list of hosts and a required destination mailbox.  When the list of hosts is present, it is a source route and indicates that the mail must be relayed to the next host on the list.  If the receiver-SMTP does not implement the relay function it may user the same reply it would for an unknown local user (550).

When mail is relayed, the relay host must remove itself from the beginning forward-path and put itself at the beginning of the reverse-path.  When mail reaches its ultimate destination (the forward-path contains only a destination mailbox), the receiver-SMTP inserts it into the destination mailbox in accordance with its host mail conventions.

For example, mail received at relay host A with arguments

  FROM:<USERX@HOSTY.ARPA>
  TO:<@HOSTA.ARPA,@HOSTB.ARPA:USERC@HOSTD.ARPA>

will be relayed on to host B with arguments

  FROM:<@HOSTA.ARPA:USERX@HOSTY.ARPA>
  TO:<@HOSTB.ARPA:USERC@HOSTD.ARPA>.

This command causes its forward-path argument to be appended to the forward-path buffer.

## DATA (DATA)

The receiver treats the lines following the command as mail data from the sender.  This command causes the mail data from this command to be appended to the mail data buffer. The mail data may contain any of the 128 ASCII character codes.

The mail data is terminated by a line containing only a period, that is the character sequence "<CRLF>.<CRLF>" (see Section 4.5.2 on Transparency).  This is the end of mail data indication.

The end of mail data indication requires that the receiver must now process the stored mail transaction information. This processing consumes the information in the reverse-path buffer, the forward-path buffer, and the mail data buffer, and on the completion of this command these buffers are cleared.  If the processing is successful the receiver must send an OK reply.  If the processing fails completely the receiver must send a failure reply.

When the receiver-SMTP accepts a message either for relaying or for final delivery it inserts at the beginning of the mail data a time stamp line.  The time stamp line indicates the identity of the host that sent the message, and the identity of the host that received the message (and is inserting this time stamp), and the date and time the message was received.  Relayed messages will have multiple time stamp lines.

When the receiver-SMTP makes the "final delivery" of a message it inserts at the beginning of the mail data a return path line.  The return path line preserves the information in the <reverse-path> from the MAIL command. Here, final delivery means the message leaves the SMTP world.  Normally, this would mean it has been delivered to the destination user, but in some cases it may be further processed and transmitted by another mail system.

Note: It is possible for the mailbox in the return path be different from the actual sender's mailbox, for example, if error responses are to be delivered a special error handling mailbox rather than the message senders.

The preceding two paragraphs imply that the final mail data will begin with a  return path line, followed by one or more time stamp lines.  These lines will be followed by the mail data header and body [2].  See Example 8.

Special mention is needed of the response and further action required when the processing following the end of mail data indication is partially successful.  This could arise if after accepting several recipients and the mail data, the receiver-SMTP finds that the mail data can be successfully delivered to some of the recipients, but it cannot be to others (for example, due to mailbox space allocation problems).  In such a situation, the response to the DATA command must be an OK reply.  But, the receiver-SMTP must compose and send an "undeliverable mail" notification message to the originator of the message.  Either a single notification which lists all of the recipients that failed to get the message, or separate notification messages must be sent for each failed recipient (see Example 7).  All undeliverable mail notification messages are sent using the MAIL command (even if they result from processing a SEND, SOML, or SAML command).

```
-------------------------------------------------------------

     Example of Return Path and Received Time Stamps

Return-Path: <@GHI.ARPA,@DEF.ARPA,@ABC.ARPA:JOE@ABC.ARPA>
Received: from GHI.ARPA by JKL.ARPA ; 27 Oct 81 15:27:39 PST
Received: from DEF.ARPA by GHI.ARPA ; 27 Oct 81 15:15:13 PST
Received: from ABC.ARPA by DEF.ARPA ; 27 Oct 81 15:01:59 PST
Date: 27 Oct 81 15:01:01 PST
From: JOE@ABC.ARPA
Subject: Improved Mailing System Installed
To: SAM@JKL.ARPA

This is to inform you that ...

                         Example 8

-------------------------------------------------------------
```

## RESET (RSET)

This command specifies that the current mail transaction is to be aborted.  Any stored sender, recipients, and mail data must be discarded, and all buffers and state tables cleared. The receiver must send an OK reply.

## NOOP (NOOP)

This command does not affect any parameters or previously entered commands.  It specifies no action other than that the receiver send an OK reply.

This command has no effect on any of the reverse-path buffer, the forward-path buffer, or the mail data buffer.

## QUIT (QUIT)

This command specifies that the receiver must send an OK reply, and then close the transmission channel.

The receiver should not close the transmission channel until it receives and replies to a QUIT command (even if there was an error).  The sender should not close the transmission channel until it send a QUIT command and receives the reply (even if there was an error response to a previous command). If the connection is closed prematurely the receiver should act as if a RSET command had been received (canceling any pending transaction, but not undoing any previously completed transaction), the sender should act as if the command or transaction in progress had received a temporary error (4xx).

# Command syntax

The commands consist of a command code followed by an argument field.  Command codes are four alphabetic characters.  Upper and lower case alphabetic characters are to be treated identically.  Thus, any of the following may represent the mail command:

MAIL    Mail    mail    MaIl    mAIl

This also applies to any symbols representing parameter values, such as "TO" or "to" for the forward-path.  Command codes and the argument fields are separated by one or more spaces. However, within the reverse-path and forward-path arguments case is important.  In particular, in some hosts the user "smith" is different from the user "Smith".

The argument field consists of a variable length character string ending with the character sequence <CRLF>.  The receiver is to take no action until this sequence is received.

Square brackets denote an optional argument field.  If the option is not taken, the appropriate default is implied.

The following are the SMTP commands:

- HELO <SP> <domain> <CRLF>

- MAIL <SP> FROM:<reverse-path> <CRLF>

- RCPT <SP> TO:<forward-path> <CRLF>

- DATA <CRLF>

- RSET <CRLF>

- SEND <SP> FROM:<reverse-path> <CRLF>

- NOOP <CRLF>

- QUIT <CRLF>

- TURN <CRLF>

# Sizes

There are several objects that have required minimum maximum sizes.  That is, every implementation must be able to receive objects of at least these sizes, but must not send objects larger than these sizes.

> [!NOTE]
> TO THE MAXIMUM EXTENT POSSIBLE, IMPLEMENTATION TECHNIQUES WHICH IMPOSE NO LIMITS ON THE LENGTH OF THESE OBJECTS SHOULD BE USED.

- user

  - The maximum total length of a user name is 64 characters.

- domain

  - The maximum total length of a domain name or number is 64 characters.

- path

  - The maximum total length of a reverse-path or forward-path is 256 characters (including the punctuation and element separators).

- command line

  - The maximum total length of a command line including thecommand word and the <CRLF> is 512 characters.

- reply line

  - The maximum total length of a reply line including the reply code and the <CRLF> is 512 characters.


- text line

  - The maximum total length of a text line including the <CRLF> is 1000 characters (but not counting the leading dot duplicated for transparency).

- recipients buffer

  - The maximum total number of recipients that must be buffered is 100 recipients.


> [!NOTE]
> TO THE MAXIMUM EXTENT POSSIBLE, IMPLEMENTATION TECHNIQUES WHICH IMPOSE NO LIMITS ON THE LENGTH OF THESE OBJECTS SHOULD BE USED.

Errors due to exceeding these limits may be reported by using the reply codes, for example:

- 500 Line too long.

- 501 Path too long

- 552 Too many recipients.

- 552 Too much mail data.
