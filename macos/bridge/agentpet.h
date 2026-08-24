#ifndef AGENTPET_H
#define AGENTPET_H

/// Poll the observation core once. Returns a JSON string owned by the caller,
/// which must release it with agentpet_free.
char *agentpet_poll(void);

/// Release a string returned by agentpet_poll.
void agentpet_free(char *ptr);

#endif
