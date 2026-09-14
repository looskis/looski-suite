# Business software suite language

This glossary defines the business terms used across the suite. It distinguishes the people operating a business from the people that business serves.

## Suite and access

**Installation**:
A deployed copy of the suite operated by a business or its administrator.

**Workspace**:
A business's shared working area, containing its members, enabled applications, and business records.
_Avoid_: Account, tenant in user-facing language.

**Application**:
A business capability a workspace can enable, such as CRM, scheduling, or forms.
_Avoid_: Plugin, subscription.

**User**:
A person with an identity used to sign in to the suite. A user may belong to more than one workspace.
_Avoid_: Contact, customer.

**Membership**:
A user's relationship to a workspace, including the access granted there.

**Permission**:
An action a member is allowed to perform, subject to the workspace and the records they may access.

**Role**:
A named collection of permissions assigned to a workspace member.

**Connected application**:
An external application authorized to act on a member's behalf within an approved workspace.
_Avoid_: Application alone, which refers to a selectable suite capability.

**Delegated access grant**:
A revocable approval allowing a connected application to perform specified actions on a member's behalf in a workspace. It cannot give the application more access than that member has.

**API scope**:
A named limit on the actions allowed through delegated access. A requested scope is not itself an approval.

**Contact**:
A person known to a business, whether a prospect, customer, supplier, or another business relationship. A contact does not need a login account.
_Avoid_: User, lead when referring only to the person.

**Organization**:
A company or other organization known to a business and associated with its contacts.
_Avoid_: Workspace when referring to an external organization.

## Intake and sales

**Form**:
A defined set of questions used to collect information from a respondent.

**Submission**:
The answers received in one response to a particular version of a form.
_Avoid_: Lead when referring only to submitted answers.

**Intake form**:
A form used to gather information as part of another business process, such as booking an appointment.

**Inquiry**:
A request or expression of interest received by the business. A contact can make multiple inquiries.

**Lead**:
A potential sales opportunity being assessed or followed up in CRM. A submission or booking becomes a lead only when the configured business workflow calls for it.
_Avoid_: Contact when referring to the sales opportunity.

**Opportunity**:
Potential business being evaluated or pursued. Its outcome describes whether that potential business was realized, lost, or abandoned.
_Avoid_: Deal when referring only to potential business.

**Deal**:
A concrete business transaction or engagement, whether live or historical, that may be used to coordinate participants, commitments, and progress. A deal may originate from an opportunity or be recorded directly; it does not necessarily mean a signed contract.
_Avoid_: Opportunity as a synonym, or completed transaction when the deal is still active.

**Pipeline**:
An ordered set of stages used to track sales progress.

**Status**:
A named state of a business record. A workspace can define custom CRM statuses; the name alone does not determine the record's business outcome.

**Tag**:
A reusable workspace label attached to a contact or another supported business record for categorization and filtering. A record may have multiple tags; a tag does not determine lifecycle state or grant access.

**Channel**:
A means of communicating with a business, such as email, SMS, or a messaging platform.

**Conversation**:
A related exchange of messages among participants. A contact may participate in several conversations, and a conversation need not be a sales opportunity.

**Message**:
An individual inbound or outbound communication within a conversation.

## Scheduling and delivery

**Appointment type**:
A bookable offering with a defined duration, availability rules, and intake requirements.

**Appointment**:
A scheduled meeting or service commitment with a time, participants, and current status.

**Booking**:
The reservation that creates an appointment. Rescheduling changes the existing appointment rather than creating an unrelated customer interaction.

**Project**:
A body of work organized for delivery, containing tasks and associated records.

**Ticket**:
A customer support request tracked through assignment and resolution.

**Contact timeline**:
A chronological view of business interactions associated with a contact, limited to records the viewer may access.

## Connected work

**Workflow**:
A configured sequence of business actions triggered by an occurrence such as a form submission or confirmed booking.

**Recipe**:
A built-in workflow template that an administrator can configure for a specific business process.

**Workflow run**:
One execution of a configured workflow for a particular originating occurrence, including its progress and outcome.
