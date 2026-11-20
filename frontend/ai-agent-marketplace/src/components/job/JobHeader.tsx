import React from 'react';
import { Link } from 'react-router-dom';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent } from '@/components/ui/Card';
import {
  DollarSign, Clock, User, CheckCircle, AlertCircle, ArrowLeft, XCircle,
} from 'lucide-react';
import type { Job } from '@/lib/types';
import { formatPrice, getSkillColor } from '@/pages/Marketplace';
import { format } from 'date-fns/format';
import { formatDistanceToNow } from 'date-fns/formatDistanceToNow';

export const statusConfig = {
  Open:       { label: 'Open for Bids',   color: 'success'     as const, icon: DollarSign },
  InProgress: { label: 'In Progress',     color: 'default'     as const, icon: Clock },
  Completed:  { label: 'Completed',       color: 'success'     as const, icon: CheckCircle },
  Disputed:   { label: 'Disputed',        color: 'destructive' as const, icon: AlertCircle },
  Cancelled:  { label: 'Cancelled',       color: 'secondary'   as const, icon: XCircle },
};

export interface JobHeaderProps {
  job: Job;
  daysLeft: number;
  isExpired: boolean;
}

/**
 * Header block for a job: title, status badge, budget, deadline and client.
 */
export const JobHeader: React.FC<JobHeaderProps> = ({ job, daysLeft, isExpired }) => {
  const status = statusConfig[job.status] ?? statusConfig.Open;
  const StatusIcon = status.icon;

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-start justify-between gap-4 mb-4">
          <div className="flex-1">
            <Link
              to="/marketplace"
              className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground mb-2"
            >
              <ArrowLeft className="h-4 w-4" /> Back to Marketplace
            </Link>
            <h1 className="text-2xl font-bold mb-2">{job.title}</h1>
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={status.color}>
                <StatusIcon className="h-3 w-3 mr-1" />
                {status.label}
              </Badge>
              {job.skills?.map((skill) => (
                <Badge key={skill} className={getSkillColor(skill)}>
                  {skill}
                </Badge>
              ))}
            </div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-bold text-primary">{formatPrice(job.budget)}</div>
            <div className="text-sm text-muted-foreground">Budget</div>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
          <div className="flex items-center gap-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span>
              {isExpired ? 'Deadline passed' : `${daysLeft} days left`}
              {' · '}
              {format(new Date(job.deadline * 1000), 'MMM d, yyyy')}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <User className="h-4 w-4 text-muted-foreground" />
            <span className="truncate">
              Client: {job.client.slice(0, 6)}…{job.client.slice(-4)}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <DollarSign className="h-4 w-4 text-muted-foreground" />
            <span>Posted {formatDistanceToNow(new Date(job.createdAt * 1000), { addSuffix: true })}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
