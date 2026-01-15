-- Add short_code column to user_redemptions
ALTER TABLE rewards.user_redemptions
ADD COLUMN short_code VARCHAR(10);

-- Create index for fast lookup by short_code
CREATE INDEX idx_user_redemptions_short_code ON rewards.user_redemptions(short_code);

-- Add unique constraint to short_code (optional but good practice, though collisions are possible with 6 chars, we handle them in code)
-- We won't enforce unique constraint on DB level for short_code to avoid complex migration issues with existing data (if any), 
-- but since this is a new system, we can try. 
-- Actually, let's just index it. The application logic will ensure uniqueness during generation.
