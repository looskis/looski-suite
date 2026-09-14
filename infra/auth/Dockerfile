FROM supabase/gotrue:v2.196.0
USER root
# Read host-owned Compose secrets, then exec Auth as its upstream supabase user.
RUN apk add --no-cache su-exec
